{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (system: let
      name = "synym";
      src = ./.;
      pkgs = import nixpkgs {
        inherit system;
      };
      crossPkgs = pkgs.pkgsCross.riscv64-embedded.buildPackages;
    in rec {
      devShells.llvm = pkgs.mkShell {
        nativeBuildInputs =
          (with pkgs; [
            qemu
            rustup
            just
          ])
          ++ (with crossPkgs; [clang gdb]);
        shellHook = ''
          export CC="riscv64-none-elf-cc -mabi=lp64d"
        '';
      };
      devShells.gcc = pkgs.mkShell {
        nativeBuildInputs =
          (with pkgs; [
            qemu
            rustup
            just
          ])
          ++ (with crossPkgs; [gcc gdb]);
        shellHook = ''
          export CC="riscv64-none-elf-gcc -mabi=lp64d"
        '';
      };
      devShells.default = devShells.llvm;
    });
}
