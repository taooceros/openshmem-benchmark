{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  env.PEER = "venus";
  env.PEER_CWD = "openshmem-benchmark";


  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    gcc15
    libclang
    nushell
  ];

  scripts."bench:rma" = {
    exec = "./run.nu bench rma";
    binary = "nu";
    package = pkgs.nushell;
    description = "Run the benchmark";
  };

  tasks."benchmark:build" = {
    exec = "cargo build --release";
    description = "Build the benchmark";
  };

  env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };
}
