{
  lib,
  stdenv,
  darwin,
  rustPlatform,
  fetchCrate,
  pkg-config,
}:

rustPlatform.buildRustPackage rec {
  pname = "esp-generate";
  version = "0.3.1";

  src = fetchCrate {
    inherit pname version;
    hash = "sha256-dSZM7J6Vus82HwKQtxvH+DlLGmu5cQ+ctWOlkB+rJfA=";
  };

  useFetchCargoVendor = true;
  cargoHash = "sha256-ncTX9cDSAf6ZGlz0utGYxkuXcx85vt3VHQzdmQhCNf0=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs =
    [
    ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [
      darwin.apple_sdk.frameworks.SystemConfiguration
    ];

  buildNoDefaultFeatures = true;
  buildFeatures = [
    "update-informer"
  ];

  cargoBuildFlags = [
  ];
  cargoTestFlags = [
  ];

  checkFlags = [
  ];

  meta = with lib; {
    description = "Tool to generate rust esp template projects";
    mainProgram = "esp-generate";
    homepage = "https://github.com/esp-rs/esp-generate";
    #changelog = "https://github.com/esp-rs/esp-generate/releases/tag/v${version}";
    #license = licenses.gpl3Only;
    maintainers = with maintainers; [ ];
  };
}
