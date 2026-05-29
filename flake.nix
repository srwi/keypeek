{
  description = "keypeek";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    pre-commit = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    #advisory-db,
    pre-commit,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      inherit (pkgs) lib;

      libPath = with pkgs;
        lib.makeLibraryPath [
          libGL
          libxkbcommon
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
          libayatana-appindicator
        ];

      craneLib = crane.mkLib pkgs;
      src = lib.cleanSourceWith {
        src = ./.;
        # filter out all non rust and still include resources
        filter = path: type:
          (builtins.match ".+-source/resources/.*$" path != null) || (craneLib.filterCargoSources path type);
        name = "source";
      };

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = false; # else openssl-sys fails

        buildInputs = with pkgs; [
          glib
          dbus
          atk
          gdk-pixbuf
          pango
          gtk3
          udev
          xdotool
          pkg-config

          libxcb
        ];

        # Additional environment variables can be set directly
        # MY_CUSTOM_VAR = "some value";
      };

      craneLibLLvmTools =
        craneLib.overrideToolchain
        (fenix.packages.${system}.stable.withComponents [
          "cargo"
          "llvm-tools"
          "rustc"
        ]);

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      keypeek = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
        }
        // {
          nativeBuildInputs = [pkgs.makeWrapper];
          postInstall = ''
            wrapProgram "$out/bin/keypeek" --prefix LD_LIBRARY_PATH : "${libPath}"
          '';
        });
    in {
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit keypeek;

        # Run clippy (and deny all warnings) on the crate source,
        # again, reusing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        #keypeek-clippy = craneLib.cargoClippy (commonArgs
        #  // {
        #    inherit cargoArtifacts;
        #    cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        #  });

        #keypeek-doc = craneLib.cargoDoc (commonArgs
        #  // {
        #    inherit cargoArtifacts;
        #  });

        # Check formatting
        #keypeek-fmt = craneLib.cargoFmt {
        #  inherit src;
        #};

        # Audit dependencies
        #keypeek-audit = craneLib.cargoAudit {
        #  inherit src advisory-db;
        #};

        # Audit licenses
        #keypeek-deny = craneLib.cargoDeny {
        #  inherit src;
        #};

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on `keypeek` if you do not want
        # the tests to run twice
        keypeek-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

        pre-commit-check = pre-commit.lib.${system}.run {
          src = ./.;
          hooks = {
            #editorconfig-checker.enable = true;
            alejandra.enable = true;
            deadnix.enable = true;
            #flake-checker.enable = true;
            statix.enable = true;
          };
        };
      };

      packages =
        {
          default = keypeek;
        }
        // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          keypeek-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs
            // {
              inherit cargoArtifacts;
            });
        };

      apps.default = flake-utils.lib.mkApp {
        drv = keypeek;
      };

      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else";
        LD_LIBRARY_PATH = libPath;

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [];
      };
    });
}
