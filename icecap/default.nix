with import ./nix;

{
  m = virt.runtime-manager;
  t = virt.veracruz-server-test;
  r = virt.run;
}
