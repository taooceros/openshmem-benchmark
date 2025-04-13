#!/usr/bin/env nu

let second_host = $env.PEER
let device = $env.DEVICE? | default "mlx5_1:1"

def execute [
    operation: record
    --epoch_size: int
    --data_size: int
    --iterations: int
    --num_pe: int
    --duration: int
    --latency,
    --num_working_set: int = 1
    --additional_args: list<string> = []
] {
    print $second_host
    let hosts = $"localhost:($num_pe),($second_host):($num_pe)"
    let operation = ( $operation | transpose | get column1)

    let latency = if $latency {
        ["--latency"]
    } else {
        []
    }

    (oshrun --wdir . --host $hosts --mca coll_ucc_enable 1 --mca coll_ucc_priority 100 -x UCC_TL_MLX5_NET_DEVICES=($device) -x UCX_NET_DEVICES=($device) ./target/release/benchmark
        ...$operation
        --epoch-size $epoch_size
        -s $data_size
        -d $duration
        -n $iterations
        -w $num_working_set
        ...$latency
        ...$additional_args)
}

def "main" [] {

}

def "main test" [
    --epoch_size (-e): int = 1
    --data_size (-s): int = 8
    --iterations (-i): int = 1000
    --operation (-o): record = { "group": "range", "operation": "all-to-all" }
    --duration: int = 5
    --num_pe: int = 16
    --num_working_set: int = 1
    --additional_args: list<string> = []
] {
    cargo build --release
    execute $operation --epoch_size $epoch_size --data_size $data_size --iterations $iterations --duration $duration --num_pe $num_pe --num_working_set $num_working_set --additional_args $additional_args
}

def merge_group [] {
    (
        $in 
        | group-by data_size epoch_size group op qps_per_instance --to-table 
        | update items {|row| $row.items.throughput | each {$in | into float} } 
        | rename --column { items: per_instance_rate, data_size: message_size, num_pe: qps_per_instance} 
        | insert total_message_rate {|row| $row.per_instance_rate | math sum } 
        | insert median_message_rate {|row| $row.per_instance_rate | math median }
    )
}

def single_bench [operation: record, epoch_size: int, data_size: int, iterations: int,  num_pe: int = 1, duration = 1, --latency, --additional_args: list = []] {
    for trial in [1 2 3 4 5] {
        try {
            print $"($trial) Running with window size: ($epoch_size), data size: ($data_size), iterations: ($iterations)"
            let output = (
                execute $operation 
                --epoch_size $epoch_size 
                --data_size $data_size 
                --iterations $iterations 
                --duration $duration 
                --latency=($latency)
                --num_pe $num_pe 
                --additional_args $additional_args 
                err>| to text)
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
                let throughput = ($message | parse "PE {pe_id}: {throughput}" | into record)
                print $throughput
                ($throughput | merge {
                    "data_size": $data_size,
                    "epoch_size": $epoch_size,
                    ...$operation,
                    "qps_per_instance": $num_pe,
                    "device": $device,
                })
            }
            
            print $record
    
            sleep 2sec

            return $record
            break
        } catch {|err|
            print $"Error occurred: ($err)"
        }
        
        print "Retrying..."
        sleep 1sec
    }
}

def nested_each [items, f: closure, args: list = [], additional_args: record = {}] {
    if ($items | length) > 0 {
        let current = $items | first
        let rest = $items | skip 1
        let records = $current | each {|item|
            let args = $args | append $item
            return (nested_each $rest $f $args $additional_args)
        } | flatten --all

        return $records
    } else {
        let result = do $f ...$args
        print "Results" $result
        return $result
    }
}

def "main bench" [] {
    cargo build --release
    
    let iterations = 500
    
    # let operations = [
    #     ["group", "op"];
    #     # ["range", "put"]
    #     # ["range", "get"]
    #     ["range", "put-non-blocking"]
    #     ["range", "get-non-blocking"]
    #     # ["range", "broadcast"]
    # ]
    let num_pes = [ 1 2 4 6 8 10 ]
    let epoch_sizes = [ 4096 ] # [ 1024 2048 4096 8192 ]
    let data_sizes = [8] # [1 8 64 128 1024 4096]
    let duration = 10

    # let records = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
    #     single_bench $operation $epoch_size $data_size $iterations $num_pe $duration
    # }

    # print $records

    let operations = [
        ["group", "op"];
        # ["range", "put"]
        # ["range", "get"]
        # ["range", "put-non-blocking"]
        # ["range", "get-non-blocking"]
        ["range", "broadcast"]
        ["range", "broadcast-non-blocking"]
    ]

    let num_pes = [ 1 2 4 8 16 32 ]

    let records = (nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
        single_bench $operation $epoch_size $data_size $iterations $num_pe $duration
    })

    print $records

    $records | merge_group | save "throughputs.json" -f

    # let num_pes = [ 1 2 4 6 8 10 ]
    # let epoch_sizes = [4096]
    # let data_sizes = [1] # Atomic operations are not supported for larger data sizes

    # let operations = [
    #     ["group", "op"];
    #     ["atomic" "fetch-add32"]
    #     ["atomic" "fetch-add64"]
    # ]

    # let records_same_location = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
    #     single_bench $operation $epoch_size $data_size $iterations $num_pe $duration []
    # }

    # let records_different_location = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
    #     single_bench $operation $epoch_size $data_size $iterations $num_pe $duration ["--use-different-location"]
    # }


    # $records_same_location | merge_group | save "throughputs-atomic-contention.json" -f
    # $records_different_location | merge_group | save "throughputs-atomic-different-location.json" -f
}

def "main bench latency" [] {
    cargo build --release

    let operations = [
        ["group", "op"];
        ["range" "get"]
        ["atomic" "atomic-i32"]
        ["range" "atomic-i64"]
    ]

    let iterations = 10000
    let epoch_sizes = [1]
    let $data_sizes = [8]
    let num_pes = [ 1 2 4 6 8 10 ]
    let duration = 10

    let records_latency = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
        single_bench $operation $epoch_size $data_size $iterations $num_pe $duration --latency
    }

    $records_latency | merge_group | save "throughputs-latency.json" -f
}

def "main bench broadcast" [] {
    cargo build --release

    let iterations = 1000
    let epoch_sizes = [4096]
    let data_sizes = [8]
    let num_pes = [ 1 2 4 8 16 32 ]
    let duration = 20

    let records_broadcast = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "broadcast" } $epoch_size $data_size $iterations $num_pe $duration
    }

    $records_broadcast | merge_group | save "throughputs-broadcast.json" -f
}

def "main bench put-get" [] {
    cargo build --release

    let iterations = 1000
    let epoch_sizes = [4096]
    let data_sizes = [8]
    let num_pes = [ 1 2 4 6 8 10 ]
    let duration = 10

    let records_uniform_put_get = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--put-ratio", "0.5"]
    }

    $records_uniform_put_get | merge_group | save "throughputs-uniform-put-get.json" -f

    let records_5_put_get = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--put-ratio", "0.05"]
    }

    $records_5_put_get | merge_group | save "throughputs-5-put-get.json" -f

    let records_one_put_one_get = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--op-sequence", "put,get"]
    }

    $records_one_put_one_get | merge_group | save "throughputs-one-put-one-get.json" -f

    let records_one_get_one_put = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--op-sequence", "get,put"]
    }

    $records_one_put_one_get | merge_group | save "throughputs-one-get-one-put.json" -f   
}

