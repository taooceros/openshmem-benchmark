cargo build --release

for window_size in [1 2 4 8 16 32 64 128] {
    for data_size in [1 2 4 8 16 32 64 128] {
        echo "Running with window size: $window_size, data size: $data_size, iterations: $iterations"
        mpirun --wdir . --hostfile hostfile -mca pml ucx --mca btl ^vader,tcp,openib,uct -x UCX_NET_DEVICES=mlx5_1:1 ./target/release/openshmem-benchmark -w $window_size -s $data_size -d 2
    }
}
mpirun --wdir . --hostfile hostfile -mca pml ucx --mca btl ^vader,tcp,openib,uct -x UCX_NET_DEVICES=mlx5_1:1 ./target/release/openshmem-benchmark