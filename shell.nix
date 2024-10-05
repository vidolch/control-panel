let
  # Pinned nixpkgs, deterministic. Last updated: 2/12/21.
  pkgs = import (fetchTarball("https://github.com/NixOS/nixpkgs/archive/refs/tags/24.05.tar.gz")) {};

  # Rolling updates, not deterministic.
  # pkgs = import (fetchTarball("channel:nixpkgs-unstable")) {};
in pkgs.mkShell {
  buildInputs = [ pkgs.cargo pkgs.rustc pkgs.rust-analyzer ];
}
