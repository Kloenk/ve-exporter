{
  system ? builtins.currentSystem
, pkgs ? import <nixpkgs> { }
, cargo ? pkgs.cargo
, makeRustPlatform ? pkgs.makeRustPlatform
, libudev ? pkgs.libudev
, pkgconfig ? pkgs.pkgconfig
, ...
}:

let
  rustPlatform = makeRustPlatform {
    rustc = cargo;
    cargo = cargo;
  };
in
rustPlatform.buildRustPackage rec {
  name = "ve-exporter-${version}";
  version = "0.1.0";
  src = ./.;
  cargoSha256 = "0f99rdjj2yd8gal7ylzpzv0ay40rv9zm3czlrgpzj5blk1yax0rc";
  #nativBuildInputs = [ libudev pkgconfig ];
  CARGO_HOME = "$(mktemp -d nix-cargo-home.XXX)";
  
}
