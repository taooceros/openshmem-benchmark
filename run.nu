#!/usr/bin/env nu

let second_host = $env.PEER
let device = $env.DEVICE? | default "mlx5_1:1"
cargo build --release

def "main bench trace2" [] {
    cargo build --release

    let files = [
        "gmm_pe_0.event_bytes.csv",
        diffusion_simulator_trace_ondemand.csv
        diffusion_simulator_trace_original.csv
    ]

    let num_pes = 1..2

    let records = nested_each [$num_pes $files] {|num_pe: int file: string|
        main run trace $file $num_pe
    }

    print $records
}

def "main run trace" [file: string, num_pes: int] {
    cargo build --release

    let hosts = $"localhost:1,($second_host):1"

    (oshrun
    -n ($num_pes * 2)
    --wdir . 
    --host $hosts 
    --mca coll_ucc_enable 0 
    --mca scoll_ucc_enable 1 
    --mca scoll_ucc_priority 100 
    -x UCC_TL_MLX5_NET_DEVICES=($device) 
    -x UCX_NET_DEVICES=($device) 
    -x UCX_RC_MLX5_DM_COUNT=0 -x UCX_DC_MLX5_DM_COUNT=0 
    ./target/release/trace-execution --trace-file $file) | lines | where {|it| $it | str starts-with "Op/s"} | get 0 | parse "Op/s: {throughput}" | get throughput | into float
}

def "main profile" [] {
    print $second_host
    let num_pe = 1
    let operation = { "group": "range", "operation": "put" }
    let latency = false
    let epoch_size = 4096
    let data_size = 8
    let duration = 10
    let iterations = 10000
    let num_working_set = 1
    let additional_args = []

    let hosts = $"localhost:($num_pe),($second_host):($num_pe)"
    let operation = ( $operation | transpose | get column1)

    let latency = if $latency {
        ["--latency"]
    } else {
        []
    }

    (oshrun --wdir . --host $hosts --mca coll_ucc_enable 0 --mca scoll_ucc_enable 1 --mca scoll_ucc_priority 100 -x UCC_TL_MLX5_NET_DEVICES=($device) -x UCX_NET_DEVICES=($device) -x UCX_RC_MLX5_DM_COUNT=0 -x UCX_DC_MLX5_DM_COUNT=0
        cargo flamegraph
        ...$operation
        --epoch-size $epoch_size
        -s $data_size
        -d $duration
        -n $iterations
        -w $num_working_set
        ...$latency
        ...$additional_args)
}

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

    (oshrun --wdir . --host $hosts --mca coll_ucc_enable 0 --mca scoll_ucc_enable 1 --mca scoll_ucc_priority 100 -x UCC_TL_MLX5_NET_DEVICES=($device) -x UCX_NET_DEVICES=($device) -x UCX_TLS=rc -x UCX_RC_MLX5_DM_COUNT=0 -x UCX_DC_MLX5_DM_COUNT=0
        ./target/release/benchmark
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
    --epoch_size (-e): int = 16
    --data_size (-s): int = 8
    --iterations (-i): int = 10000
    --operation (-o): record = { "group": "range", "operation": "put" }
    --duration: int = 10
    --num_pe: int = 2
    --num_working_set: int = 1
    --additional_args: list<string> = []
    --latency
] {
    cargo build --release
    execute $operation --epoch_size $epoch_size --data_size $data_size --iterations $iterations --duration $duration --num_pe $num_pe --num_working_set $num_working_set --additional_args $additional_args --latency=$latency
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
            if ($throughput | length) == 0 {
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

def pow [base: int] : [
    list<int> -> list<int>
    range -> list<int>
] {
    $in | each {|x|
        $base ** $x
    }
}

def "main bench" [] {
    main bench rma
    main bench atomic
    main bench latency
    main bench broadcast
    main bench alltoall
    main bench ycsb
    main bench trace
}

def "main bench rma" [] {
    
    let iterations = 500
    
    let operations = [
        ["group", "op"];
        # ["range", "put"]
        # ["range", "get"]
        ["range", "put-non-blocking"]
        ["range", "get-non-blocking"]
        # ["range", "broadcast"]
    ]
    let num_pes = [1]
    let epoch_sizes = [ 1024 ] # [ 1024 2048 4096 8192 ]
    let data_sizes = [1 2 4 8 16 32 64 128 256 512 1024 2048 4096 8192 16384 32768 65536] # [1 8 64 128 1024 4096]
    let duration = 10

    let records = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
        single_bench $operation $epoch_size $data_size $iterations $num_pe $duration
    }

    print $records

    $records | merge_group | save "throughputs.json" -f

    
}

def "main bench atomic" [] {
    let iterations = 500
    let num_pes = 1..10
    let epoch_sizes = [4096]
    let data_sizes = [1] # Atomic operations are not supported for larger data sizes
    let duration = 15

    let operations = [
        ["group", "op"];
        ["atomic" "fetch-add32"]
        ["atomic" "fetch-add64"]
    ]

    let records_same_location = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
        single_bench $operation $epoch_size $data_size $iterations $num_pe $duration --additional_args []
    }

    let records_different_location = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
        single_bench $operation $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--use-different-location"]
    }


    $records_same_location | merge_group | save "throughputs-atomic-contention.json" -f
    $records_different_location | merge_group | save "throughputs-atomic-different-location.json" -f
}

def "main bench latency" [] {
    cargo build --release

    let operations = [
        ["group", "op"];
        ["range" "get"]
        ["atomic" "fetch-add32"]
        ["range" "fetch-add64"]
    ]

    let iterations = 10000
    let epoch_sizes = [1]
    let $data_sizes = [8]
    let num_pes = 1..10
    let duration = 10

    let records_latency = nested_each [$operations $num_pes $epoch_sizes $data_sizes] {|$operation: record num_pe: int epoch_size: int data_size: int|
        single_bench $operation $epoch_size $data_size $iterations $num_pe $duration --latency
    }

    $records_latency | merge_group | save "latency.json" -f
}

def "main bench broadcast" [] {
    cargo build --release

    let iterations = 1000
    let epoch_sizes = [4096]
    let data_sizes = [8]
    let num_pes = (0..5 | pow 2)
    let duration = 20

    let records_broadcast = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "broadcast" } $epoch_size $data_size $iterations $num_pe $duration
    }

    $records_broadcast | merge_group | save "throughputs-broadcast.json" -f
}


def "main bench alltoall" [] {
    cargo build --release

    let iterations = 100
    let epoch_sizes = [4096]
    let data_sizes = [8]
    let num_pes = 1..10
    let duration = 20

    let records_broadcast = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "all-to-all" } $epoch_size $data_size $iterations $num_pe $duration
    }

    $records_broadcast | merge_group | save "throughputs-all-to-all.json" -f
}


def "main bench ycsb" [] {
    cargo build --release

    let iterations = 1000
    let epoch_sizes = [4096]
    let data_sizes = [8]
    let num_pes = 1..10
    let duration = 10

    let records_ycsb_a = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--put-ratio", "0.5"]
    }

    $records_ycsb_a | merge_group | save "throughputs-ycsb-a.json" -f

    let records_ycsb_b = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--put-ratio", "0.05"]
    }

    $records_ycsb_b | merge_group | save "throughputs-ycsb-b.json" -f

    let records_ycsb_f = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
        single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--op-sequence", "get,put,get"]
    }

    $records_ycsb_f | merge_group | save "throughputs-ycsb-f.json" -f   
}

def "main bench trace" [] {
    cargo build --release

    ls spmm*.json | each {|trace_file|
        let iterations = 1000
        let epoch_sizes = [4096]
        let data_sizes = [8]
        let num_pes = 1..10
        let duration = 10

        let records_uniform_put_get = nested_each [$num_pes $epoch_sizes $data_sizes] {|num_pe: int epoch_size: int data_size: int|
            single_bench { "group": "range", "op": "put-get" } $epoch_size $data_size $iterations $num_pe $duration --additional_args ["--op-sequence-file", $trace_file.name]
        }

        $records_uniform_put_get | merge_group | save $"throughputs-trace-($trace_file.name)" -f
    }
}

def "main transform-trace" [] {
    ls throughputs.json | each {|trace_file|
        let trace_file = $trace_file.name
        let trace = (open $trace_file)
        let trace = $trace | each {|row|
            if $row == 0 {
                "Get"
            } else {
                "Put"
            }
        }

        $trace | save $trace_file -f
    }
}