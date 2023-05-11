{ naersk, gnumake, src, lib, pkg-config, fetchFromGitHub}:

naersk.buildPackage {
  pname = "view";
  version = "0.0.0";

  src = fetchFromGitHub {
    owner = "marcelcoding";
    repo = "view";
    rev = "v0.0.0";
    sha256 = "sha256-L8B9ZtBgkxFRT8gxpq9UTXEx6toEj0QrmH8fvgpLVDY=";
  };

  cargoSha256 = lib.fakeSha256;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ ];

  meta = {
    description = "Selfhosted Replacement for Cloudflarepages";
    homepage = "https://github.com/marcelcoding/view";
  };
}
