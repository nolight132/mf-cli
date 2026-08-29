{
  lib,
  stdenv,
  kernel,
}:

stdenv.mkDerivation {
  pname = "minifuse-mod";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ../kmod;
    fileset = lib.fileset.unions [
      ../kmod/Makefile
      ../kmod/minifuse_mod.c
    ];
  };

  nativeBuildInputs = kernel.moduleBuildDependencies;

  makeFlags = [
    "KDIR=${kernel.dev}/lib/modules/${kernel.modDirVersion}/build"
  ];

  hardeningDisable = [
    "pic"
    "format"
  ];

  enableParallelBuilding = true;

  installPhase = ''
    runHook preInstall
    install -Dm444 minifuse_mod.ko -t $out/lib/modules/${kernel.modDirVersion}/extra
    runHook postInstall
  '';

  meta = {
    description = "Kernel module that bypasses usbfs to control an Arturia MiniFuse without audio dropouts";
    homepage = "https://github.com/nolight132/mf-cli";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
  };
}
