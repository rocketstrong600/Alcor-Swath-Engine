{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, crane }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.mkLib pkgs;
        
        # 1. Build only the dependencies (caches heavily)
        cargoArtifacts = craneLib.buildDepsOnly {
          src = craneLib.cleanCargoSource (craneLib.path ./.);
        };

        # 2. Build the actual crate
        my-crate = craneLib.buildPackage {
          src = craneLib.cleanCargoSource (craneLib.path ./.);
          inherit cargoArtifacts;
        };
      in
      {
        packages.default = my-crate;
        devShells.default = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy rust-analyzer vulkan-loader vulkan-validation-layers vulkan-tools-lunarg libxkbcommon wayland shader-slang libX11 libXcursor libXi];
          packages = [ vulkan-tools renderdoc mangohud harper ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [ vulkan-loader libxkbcommon wayland libX11 libXcursor libXi ]}:$LD_LIBRARY_PATH
            export VK_LAYER_PATH=${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d:$VK_LAYER_PATH
            
            # Transparently wrap cargo with nixGL on non-NixOS systems
            if [ ! -f /etc/NIXOS ]; then
              cargo() {
                nix run --impure github:nix-community/nixGL -- cargo "$@"
              }
            fi
          '';
        };
      }
    );
}
