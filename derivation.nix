{ naersk, gnumake, src, lib, pkg-config}:

naersk.buildPackage {
  pname = "view";
  version = "0.0.0";

  src = ./.;

  cargoSha256 = lib.fakeSha256;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ ];

  meta = {
    description = "Selfhosted Replacement for Cloudflarepages";
    homepage = "https://github.com/marcelcoding/view";
  };
}
