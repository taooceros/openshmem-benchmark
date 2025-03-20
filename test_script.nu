cargo build --release

mut records = []

for data_size in [1024 2048 4096 8192 16384] {
    for window_size in [1 2 4 8 16 32 64 128] {
        for trial in [1 2 3 4 5] {
            try {
                print $"($trial) Running with window size: $window_size, data size: $data_size, iterations: $iterations"
                let output = mpirun --wdir . --hostfile hostfile -mca pml ucx --mca btl ^vader,tcp,openib,uct -x UCX_NET_DEVICES=mlx5_1:1 ./target/release/openshmem-benchmark -w $window_size -s $data_size -d 10 -n 100000 err>| to text
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