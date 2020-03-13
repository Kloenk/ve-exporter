{
  system ? builtins.currentSystem
, pkgs ? import <nixpkgs> { }
, cargo ? pkgs.cargo
, makeRustPlatform ? pkgs.makeRustPlatform
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
  cargoSha256 = "0jgaz43wzrk01vssqrydqqka7flhnv6k7p4ajkqmv72ib5h9djh8";
  buildInputs = [ ];
  CARGO_HOME = "$(mktemp -d nix-cargo-home.XXX)";
  
}
