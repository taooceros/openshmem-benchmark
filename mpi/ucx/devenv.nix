{ pkgs, ... }: {
  overlays = [
    (final: prev: {
      ucx = prev.ucx.overrideAttrs (old: {
        version = "unstable";
        src = pkgs.fetchFromGitHub {
          owner = "openucx";
          repo = "ucx";
          rev = "master";
          hash = "sha256-zZ7cTnDp+xDWR6krhSlDiqH7X0OwDZKCw/0Qrsu+gvY=";
        };
      });
    })
  ];

  packages = [
    pkgs.ucx
  ];
}