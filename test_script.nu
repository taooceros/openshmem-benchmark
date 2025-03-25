#!/usr/bin/env nu

let second_host = $env.SECOND_HOST

def execute [epoch_size: int, data_size: int, iterations: int, num_pe: int = 1, duration = 5] {
    print $second_host
    let hosts = $"localhost:($num_pe),($second_host):($num_pe)"
    mpirun --wdir . --host $hosts -mca pml ucx --mca btl ^vader,tcp,openib,uct -x UCX_NET_DEVICES=mlx5_1:1 ./target/release/benchmark --epoch-size $epoch_size -s $data_size -d $duration -n $iterations --num-pe $num_pe
}

def "main" [] {

}

def "main test" [] {
    cargo build --release
    execute 4 1024 10000 4
}

def "main bench" [] {
    cargo build --release

    mut records = []
    
    let iterations = 100000
    
    for data_size in [1024 2048 4096 8192 16384] {
        for window_size in [1 2 4 8 16 32 64 128] {
            for trial in [1 2 3 4 5] {
                try {
                    print $"($trial) Running with window size: ($window_size), data size: ($data_size), iterations: ($iterations)"
                    let output = execute $window_size $data_size $iterations err>| to text
                    print $output
                    let output_lines = $output | lines
                    let throughput = ($output_lines | where ($it starts-with "Final throughput:") | first | split row " " | get 2)
                    print $"[($data_size); ($window_size)] Throughput: ($throughput) M/s"
                    let record = {
                        "data_size": $data_size,
                        "window_size": $window_size,
                        "throughput": $throughput
                    }
            
                    $records = $records | append $record
                    break
                } catch {|err|
                    print $"Error occurred: ($err)"
                }
                
                print "Retrying..."
                sleep 1sec
            }
            
        }
    }
    
    $records | to csv | save "throughputs.csv"
}

