{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  ucx-package = pkgs.stdenv.mkDerivation {
    pname = "ucx";
    version = "unstable";
    src = inputs.ucx;

    nativeBuildInputs = with pkgs; [
      pkg-config
      git
      gcc15
      doxygen
      autoreconfHook
    ];

    buildInputs = with pkgs; [
      libbfd  
      libiberty
      numactl
      perl
      rdma-core
      zlib
    ];

    configureFlags = [
      "--with-rdmacm=${lib.getDev pkgs.rdma-core}"
      "--with-dc"
      "--with-rc"
      "--with-dm"
      "--with-verbs=${lib.getDev pkgs.rdma-core}"
    ];

    enableParallelBuilding = true;
  };
in
{
  languages = {
    cplusplus = {
      enable = true;
    };
    c = {
      enable = true;
    };
  };

  packages = with pkgs; [
    rdma-core
    ucx-package
  ];
}