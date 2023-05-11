{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # needed to set up the schema in test vm database
    tlms-rs = {
      url = "github:tlm-solutions/tlms.rs";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    utils = {
      url = "github:numtide/flake-utils";
    };
  };

  outputs = inputs@{ self, nixpkgs, naersk, tlms-rs, utils, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          package-latest = pkgs.callPackage ./derivation.nix {
            naersk = naersk.lib.${system};
          };

          package-stable = pkgs.callPackage ./derivation-stable.nix {
            naersk = naersk.lib.${system};
          };

          test-vm-pkg = self.nixosConfigurations.view-mctest.config.system.build.vm;

        in
        rec {
          checks = packages;
          packages = {
            view = package-latest;
            view-stable = package-stable;
            default = package-latest;
          };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs = (with packages.default; nativeBuildInputs ++ buildInputs) ++ [
              # python for running test scripts
              (pkgs.python3.withPackages (p: with p; [
                requests
              ]))
            ];
          };
        }
      ) // {
      overlays.default = final: prev: {
        inherit (self.packages.${prev.system})
          view view-stable;
      };

      nixosModules = rec {
        default = view;
        view = import ./nixos-module;
      };

      hydraJobs =
        let
          hydraSystems = [
            "x86_64-linux"
            "aarch64-linux"
          ];
        in
        builtins.foldl'
          (hydraJobs: system:
            builtins.foldl'
              (hydraJobs: pkgName:
                nixpkgs.lib.recursiveUpdate hydraJobs {
                  ${pkgName}.${system} = self.packages.${system}.${pkgName};
                }
              )
              hydraJobs
              (builtins.attrNames self.packages.${system})
          )
          { }
          hydraSystems;
    };
}
