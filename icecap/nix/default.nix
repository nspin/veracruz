let
  icecapLocal = import ../../..icecap/nix;
  icecapRemote = import ../../../icecap/nix;

  icecap = icecapRemote;

  plats = icecap.none.icecap.byIceCapPlat (plat:
    with icecap.instances.${plat}; mkBasicInstance configs.icecap ./instance.nix);

in plats // {
  inherit icecap;
}
