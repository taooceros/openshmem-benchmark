#!/usr/bin/env nu

let second_host = $env.SECOND_HOST

def execute [epoch_size: int, data_size: int, iterations: int, num_pe: int = 1, duration = 2] {
    print $second_host
    let hosts = $"localhost:($num_pe),($second_host):($num_pe)"
    mpirun --wdir . --host $hosts -mca pml ucx --mca btl ^vader,tcp,openib,uct -x UCX_NET_DEVICES=mlx5_1:1 ./target/release/benchmark --epoch-size $epoch_size -s $data_size -d $duration -n $iterations
}

def "main" [] {

}

def "main test" [] {
    cargo build --release
    execute 4 1024 10000 4
}

def single_bench [epoch_size: int, data_size: int, iterations: int, num_pe: int = 1, duration = 2] {
    for trial in [1 2 3 4 5] {
        try {
            print $"($trial) Running with window size: ($epoch_size), data size: ($data_size), iterations: ($iterations)"
            let output = execute $epoch_size $data_size $iterations $num_pe $duration err>| to text
            print $output
            let output_lines = $output | lines
            let throughput = ($output_lines | where ($it starts-with "PE"))
            if len($throughput) == 0 {
                print "No throughput found in output."
                print "Output: ($output)"
                sleep 1sec
                continue
            }

            let record = $throughput | each {|message|
                let throughput = ($message | parse "PE {pe_id}: {throughput} messages/second" | into record)
                ($throughput | merge {
                    "data_size": $data_size,
                    "epoch_size": $epoch_size,
                })
            }
            
            print $record
    
            return $record
            break
        } catch {|err|
            print $"Error occurred: ($err)"
        }
        
        print "Retrying..."
        sleep 1sec
    }
}

def nested_each [items: list<list>, f: closure, args: list = []] {
    if ($items | length) > 0 {
        let current = $items | first
        let rest = $items | skip 1
        let records = $current | each {|item|
            let args = $args | append $item
            return (nested_each $rest $f $args)
        } | flatten --all
        
        print "Records" $records

        return $records
    } else {
        let result = do $f ...$args
        print "Results" $result
        return $result
    }
}

def "main bench" [] {
    cargo build --release
    
    let iterations = 100000
    
    let num_pes = [1 2 4 8 16 32 64]
    let epoch_sizes = [1 2 4 8 16 32 64 128 256 512 1024]
    let data_sizes = [1 2 4 8 16 32 64 128 256 512 1024 2048 4096 8192 16384]

    let records = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe, epoch_size, data_size|
        single_bench $epoch_size $data_size $iterations $num_pe
    }

    print $records

    $records | to csv | save "throughputs.csv"
}

