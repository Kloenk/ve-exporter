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
  cargoSha256 = "18w33i3bb75xzrrlkq3jxy5cgvyc3kdq82iyv06af9k92mdxljc1";
  #nativBuildInputs = [ libudev pkgconfig ];
  CARGO_HOME = "$(mktemp -d nix-cargo-home.XXX)";
  
}
