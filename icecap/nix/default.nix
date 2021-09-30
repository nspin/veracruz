let
  icecapLocal = ../../../icecap;

  icecapRemote = builtins.fetchGit rec {
    url = "https://gitlab.com/arm-research/security/icecap/icecap.git";
    ref = "veracruz";
    rev = "ef90f3f4afd02ed3a347b6497da3d3a07bb20f8b";
    submodules = true;
  };

  # icecapSource = icecapRemote;
  icecapSource = icecapLocal;

  icecap = import icecapSource;

  plats = with icecap; lib.flip lib.mapAttrs pkgs.none.icecap.configured (_: configured:
    import ./instance.nix {
      inherit lib pkgs configured;
    }
  );

in icecap // plats
