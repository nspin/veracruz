let
  icecapLocal = ../../../icecap;

  icecapRemote = builtins.fetchGit rec {
    url = "https://gitlab.com/arm-research/security/icecap/icecap.git";
    ref = "veracruz";
    rev = "8a2e8f6765baef8adc5d03bd8452b3ccec4e84f5";
    submodules = true;
  };

  # icecapSource = icecapRemote;
  icecapSource = icecapLocal;

  icecap = import (icecapSource + "/nix");

  plats = icecap.none.icecap.byIceCapPlat (plat:
    with icecap.instances.${plat}; mkBasicInstance configs.icecap ./instance.nix);

in plats // {
  inherit icecap;
}
