{ pkgs, ... }:
{
  overlays = [
    (final: prev: {
      mpi = prev.openmpi.overrideAttrs (old: {
        version = "unstable";
        src = pkgs.fetchFromGitHub {
          owner = "open-mpi";
          repo = "ompi";
          rev = "05e66a6b27735cfea353e524a5f9c19f0cf8bd18";
          fetchSubmodules = true;
          hash = "sha256-SHD0+5NNZ9m4qrqxOt32n+iamQ8b/54lQekU97YLDBs=";
        };

        nativeBuildInputs = with pkgs; (old.nativeBuildInputs or [ ]) ++ [
          autoconf
          automake
          libtool
          python3
          git
          flex
          sphinx
        ];

        postPatch = ''
          patchShebangs ./

          ./autogen.pl
        '';

        outputs = [ "out" "dev" ];

      });
    })
  ];

  packages = [
    pkgs.mpi
  ];
}
