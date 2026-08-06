{
  inputs = {
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };
  outputs =
    {
      self,
      crane,
      fenix,
      flake-utils,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      (
        let
          pkgs = nixpkgs.legacyPackages.${system};
          rust-toolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-K8/aNzEwNFy5A+HIFCFhHbilHJezC3HqOc9YItLeZ7c=";
          };
          crane-lib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
        in
        {
          packages.default = crane-lib.buildPackage {
            src = crane-lib.cleanCargoSource ./.;
          };
          devShells.default = crane-lib.devShell { };
          formatter = pkgs.nixfmt;
        }
      )
    );
}
