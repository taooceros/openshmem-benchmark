cargo build --release
mpirun --wdir . --hostfile hostfile -mca pml ucx --mca btl ^vader,tcp,openib,uct -x UCX_NET_DEVICES=mlx5_1:1 ./target/release/openshmem-benchmark --num-iterations 1000