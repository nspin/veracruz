{ icecapPlat, instance }:

{ config, pkgs, lib, ... }:

let

in

{
  config = {

    net.interfaces = lib.optionalAttrs (icecapPlat == "virt") {
      eth1 = {};
    };

    initramfs.mntCommands = ''
      ${{
        virt = ''
          mount -t 9p -o trans=virtio,version=9p2000.L,ro store $nix_store_mnt
        '';
        rpi4 = ''
          echo -n 'mounting nix store... '
          mount -t 9p -o trans=tcp,version=9p2000.L,cache=loose,port=${toString config.rpi4._9p.port} ${config.rpi4._9p.addr} $nix_store_mnt
          echo done
        '';
      }.${icecapPlat}}

      mkdir -p $target_root/etc
      cp /etc/resolv.conf $target_root/etc
    '';

    env.extraPackages = with pkgs; [
      icecap.icecap-host
    ];

    initramfs.extraNextInit = ''
      mount -t debugfs none /sys/kernel/debug/

    '' + lib.optionalString (icecapPlat == "rpi4") ''
      (
        cd /sys/devices/system/cpu/cpu0/cpufreq/
        echo userspace > scaling_governor
        echo 1500000 > scaling_setspeed
      )
    '' + ''

      mkdir /x
      cp ${instance.db} /x/proxy-attestation-server.db

      ifconfig lo 127.0.0.1
      # route add 127.0.0.1
    '';

  };
}
