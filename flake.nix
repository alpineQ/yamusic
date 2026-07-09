{
  description = "yamusic — Yandex Music TUI (alpineQ fork: MPRIS, waybar lyrics, disk cache)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      runtimeLibs = pkgs.lib.makeLibraryPath [ pkgs.vulkan-loader ];
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "yamusic";
        version = "0.1.0";

        src = ./.;

        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          makeWrapper
          rustPlatform.bindgenHook
        ];

        buildInputs = with pkgs; [
          dbus
          alsa-lib
          openssl
          chafa
          glib
        ];

        # wgpu loads the Vulkan loader at runtime; the actual ICD comes from
        # /run/opengl-driver/lib on NixOS.
        postInstall = ''
          wrapProgram $out/bin/yamusic \
            --prefix LD_LIBRARY_PATH : "${runtimeLibs}:/run/opengl-driver/lib"
        '';

        doCheck = false;

        meta = with pkgs.lib; {
          description = "Terminal client for Yandex Music (fork: MPRIS, waybar lyrics, disk cache)";
          homepage = "https://github.com/alpineQ/yamusic";
          license = licenses.gpl3Only;
          mainProgram = "yamusic";
          platforms = platforms.linux;
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [ pkg-config cmake ];
        buildInputs = with pkgs; [ dbus alsa-lib openssl chafa glib vulkan-loader ];
        LD_LIBRARY_PATH = "${runtimeLibs}:/run/opengl-driver/lib";
      };
    };
}
