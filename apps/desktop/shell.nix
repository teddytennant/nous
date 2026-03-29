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
  ];
}
