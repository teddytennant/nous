{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    pkg-config
    glib
    gdk-pixbuf
    gtk3
    webkitgtk_4_1
    libsoup_3
    pango
    cairo
    atk
    harfbuzz
    openssl
    libayatana-appindicator
  ];

  # libayatana-appindicator and friends are loaded at runtime via dlopen, so
  # the linker would never see them otherwise. Surface them through
  # LD_LIBRARY_PATH so `cargo tauri dev` can find the .so files.
  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.libayatana-appindicator}/lib:${pkgs.gtk3}/lib:${pkgs.glib.out}/lib:''${LD_LIBRARY_PATH:-}"
  '';
}
