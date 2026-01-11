{
  description = "A reproducible Neovim environment for Rust development";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rustaceanvim-flake.url = "github:mrcjkb/rustaceanvim";
  };
  outputs = { nixpkgs, rustaceanvim-flake, ... }: {
    # Define a development shell
    devShells.x86_64-linux.default = let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in pkgs.mkShell {
      packages = with pkgs; [
        # Install Neovim itself
        neovim
        # Install Rust toolchain and rust-analyzer
        rustc
        cargo
        clang
        pkg-config
        llvmPackages.libclang.lib
        rust-analyzer
        # Install the debug adapter (codelldb)
        rustaceanvim-flake.packages.${system}.codelldb
        # Other useful tools like tree-sitter for Rust
        # (tree-sitter.withPlugins (p: [ p.rust ]))
        git # Flakes needs git internally
      ];
      LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib";
      LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
    };
  };
}
