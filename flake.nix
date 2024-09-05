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
    in rec {
      devShells.llvm = pkgs.mkShell {
        buildInputs = with pkgs.pkgsCross.riscv64-embedded.buildPackages; [clang just gdb];
        nativeBuildInputs = with pkgs; [
          qemu
        ];
        shellHook = ''
          export CC="riscv64-none-elf-cc -mabi=lp64d"
        '';
      };
      devShells.gcc = pkgs.mkShell {
        buildInputs = with pkgs.pkgsCross.riscv64-embedded.buildPackages; [gcc just gdb];
        nativeBuildInputs = with pkgs; [
          qemu
        ];
        shellHook = ''
          export CC="riscv64-none-elf-gcc -mabi=lp64d"
        '';
      };
      devShells.default = devShells.llvm;
    });
}
