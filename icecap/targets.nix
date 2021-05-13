with import ./nix;

{
  crates = virt.icecapCrates;
  runtime-manager = virt.runtime-manager;
  veracruz-server-test = virt.veracruz-server-test;
  veracruz-test = virt.veracruz-test;
  test-system-stale = virt.run;
}
