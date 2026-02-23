{
  description = "Bash compatibility layer for fish shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "reef";
          version = "0.3.0";

          src = ../..;

          cargoLock.lockFile = ../../Cargo.lock;

          nativeBuildInputs = [ pkgs.installShellFiles ];

          postInstall = ''
            # Fish functions → vendor_functions.d
            install -Dm644 fish/functions/export.fish $out/share/fish/vendor_functions.d/export.fish
            install -Dm644 fish/functions/unset.fish $out/share/fish/vendor_functions.d/unset.fish
            install -Dm644 fish/functions/declare.fish $out/share/fish/vendor_functions.d/declare.fish
            install -Dm644 fish/functions/local.fish $out/share/fish/vendor_functions.d/local.fish
            install -Dm644 fish/functions/readonly.fish $out/share/fish/vendor_functions.d/readonly.fish
            install -Dm644 fish/functions/shopt.fish $out/share/fish/vendor_functions.d/shopt.fish
            install -Dm644 fish/functions/fish_command_not_found.fish $out/share/fish/vendor_functions.d/fish_command_not_found.fish

            # conf.d
            install -Dm644 fish/conf.d/reef.fish $out/share/fish/vendor_conf.d/reef.fish
          '';

          meta = with pkgs.lib; {
            description = "Bash compatibility layer for fish shell";
            homepage = "https://github.com/ZStud/reef";
            license = licenses.mit;
            maintainers = [ ];
            mainProgram = "reef";
          };
        };

        packages.reef-tools = pkgs.stdenv.mkDerivation {
          pname = "reef-tools";
          version = "0.3.0";

          src = ../..;

          installPhase = ''
            # Tool wrappers → vendor_functions.d
            install -Dm644 fish/functions/tools/grep.fish $out/share/fish/vendor_functions.d/grep.fish
            install -Dm644 fish/functions/tools/find.fish $out/share/fish/vendor_functions.d/find.fish
            install -Dm644 fish/functions/tools/sed.fish $out/share/fish/vendor_functions.d/sed.fish
            install -Dm644 fish/functions/tools/du.fish $out/share/fish/vendor_functions.d/du.fish
            install -Dm644 fish/functions/tools/ps.fish $out/share/fish/vendor_functions.d/ps.fish
            install -Dm644 fish/functions/tools/ls.fish $out/share/fish/vendor_functions.d/ls.fish
            install -Dm644 fish/functions/tools/cat.fish $out/share/fish/vendor_functions.d/cat.fish

            # conf.d
            install -Dm644 fish/conf.d/reef-tools.fish $out/share/fish/vendor_conf.d/reef-tools.fish
          '';

          meta = with pkgs.lib; {
            description = "Modern CLI tool wrappers for fish — grep→rg, find→fd, sed→sd, ls→eza, cat→bat, cd→zoxide";
            homepage = "https://github.com/ZStud/reef";
            license = licenses.mit;
            maintainers = [ ];
          };
        };
      }
    );
}
