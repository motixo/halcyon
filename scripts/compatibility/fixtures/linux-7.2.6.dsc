-----BEGIN PGP SIGNED MESSAGE-----
Hash: SHA512

Format: 3.0 (native)
Source: linux
Binary: linux-headers-7.2.6-76070206, linux-tools-7.2.6-76070206, linux-cloud-tools-7.2.6-76070206, linux-headers-generic, linux-image-generic, linux-tools-generic, linux-cloud-tools-generic, linux-generic, linux-system76, linux-libc-dev, linux-tools-common, linux-cloud-tools-common, linux-tools-host, linux-source-7.2.6, linux-bpf-dev, bpftool, linux-perf, linux-image-unsigned-7.2.6-76070206-generic, linux-image-unsigned-7.2.6-76070206-generic-dbgsym, linux-image-7.2.6-76070206-generic, linux-image-7.2.6-76070206-generic-dbgsym, linux-modules-7.2.6-76070206-generic, linux-headers-7.2.6-76070206-generic, linux-lib-rust-7.2.6-76070206-generic, linux-tools-7.2.6-76070206-generic, linux-cloud-tools-7.2.6-76070206-generic, linux-buildinfo-7.2.6-76070206-generic, linux-image-7.2.6-76070206-generic-64k, linux-image-7.2.6-76070206-generic-64k-dbgsym, linux-modules-7.2.6-76070206-generic-64k, linux-headers-7.2.6-76070206-generic-64k, linux-lib-rust-7.2.6-76070206-generic-64k,
 linux-tools-7.2.6-76070206-generic-64k, linux-cloud-tools-7.2.6-76070206-generic-64k,
 linux-buildinfo-7.2.6-76070206-generic-64k
Architecture: all amd64 arm64 ppc64el s390x powerpc riscv64
Version: 7.2.6-76070206.202609141300~1792433904~24.04~141c6f5
Maintainer: Ubuntu Kernel Team <kernel-team@lists.ubuntu.com>
Standards-Version: 3.9.4.0
Vcs-Git: git://git.launchpad.net/~ubuntu-kernel/ubuntu/+source/linux/+git/noble
Build-Depends: gcc-13:native, gcc-13-aarch64-linux-gnu [arm64] <cross>, gcc-13-arm-linux-gnueabihf [armhf] <cross>, gcc-13-powerpc64le-linux-gnu [ppc64el] <cross>, gcc-13-riscv64-linux-gnu [riscv64] <cross>, gcc-13-s390x-linux-gnu [s390x] <cross>, gcc-13-x86-64-linux-gnu [amd64] <cross>, autoconf <!stage1>, automake <!stage1>, bc <!stage1>, bindgen-0.71:native [amd64 arm64 armhf ppc64el riscv64 s390x], bison <!stage1>, clang-18:native [amd64 arm64 armhf ppc64el riscv64 s390x], cpio, curl <!stage1>, debhelper-compat (= 10), default-jdk-headless:native <!stage1>, dkms <!stage1>, flex <!stage1>, gawk <!stage1>, java-common <!stage1>, kmod <!stage1>, libaudit-dev <!stage1>, libbpf-dev <!stage1>, libcap-dev <!stage1>, libdebuginfod-dev [amd64 arm64 armhf ppc64el s390x riscv64] <!stage1>, libdw-dev <!stage1>, libelf-dev <!stage1>, libiberty-dev <!stage1>, libllvm18 <!stage1>, liblzma-dev <!stage1>, libnewt-dev <!stage1>, libnl-3-dev, libnl-genl-3-dev, libnuma-dev [amd64 arm64 ppc64el s390x] <!stage1>, libpci-dev <!stage1>, libssl-dev <!stage1>, libstdc++-dev, libtool <!stage1>, libtraceevent-dev [amd64 arm64 armhf ppc64el s390x riscv64] <!stage1>, libtracefs-dev [amd64 arm64 armhf ppc64el s390x riscv64] <!stage1>, libudev-dev <!stage1>, libunwind8-dev [amd64 arm64 armhf ppc64el] <!stage1>, llvm <!stage1>, llvm-18-tools:native <!stage1>, makedumpfile:native [amd64] <!stage1>, openssl <!stage1>, dwarves (>= 1.21) [amd64 arm64 armhf ppc64el s390x riscv64] <!stage1>, pkg-config <!stage1>, python3:native <!stage1>, python3-dev:native <!stage1>, libpython3-dev <!stage1> <cross>, python3-setuptools, rsync [!i386] <!stage1>, rust-1.91-src:native [amd64 arm64 armhf ppc64el riscv64 s390x], rustc-1.91:native (>= 1.82) [amd64 arm64 armhf ppc64el riscv64 s390x], rustfmt-1.91:native [amd64 arm64 armhf ppc64el riscv64 s390x], uuid-dev <!stage1>, zstd <!stage1>, bpftool:native [amd64 arm64 armhf ppc64el riscv64 s390x] <!stage1 cross>
Build-Depends-Indep: asciidoc <!stage1>, bzip2 <!stage1>, python3-docutils <!stage1>, sharutils <!stage1>, xmlto <!stage1>
Package-List:
 bpftool deb devel optional arch=amd64,arm64,ppc64el,riscv64,s390x
 linux-bpf-dev deb devel optional arch=amd64,arm64,ppc64el,riscv64,s390x
 linux-buildinfo-7.2.6-76070206-generic deb kernel optional arch=amd64,arm64,ppc64el,s390x profile=!stage1
 linux-buildinfo-7.2.6-76070206-generic-64k deb kernel optional arch=arm64 profile=!stage1
 linux-cloud-tools-7.2.6-76070206 deb devel optional arch=amd64 profile=!stage1
 linux-cloud-tools-7.2.6-76070206-generic deb devel optional arch=amd64,arm64,ppc64el,s390x profile=!stage1
 linux-cloud-tools-7.2.6-76070206-generic-64k deb devel optional arch=arm64 profile=!stage1
 linux-cloud-tools-common deb kernel optional arch=all profile=!stage1
 linux-cloud-tools-generic deb kernel optional arch=amd64 profile=!stage1
 linux-generic deb kernel optional arch=amd64,arm64,powerpc,ppc64el,s390x profile=!stage1
 linux-headers-7.2.6-76070206 deb devel optional arch=all profile=!stage1
 linux-headers-7.2.6-76070206-generic deb devel optional arch=amd64,arm64,ppc64el,s390x profile=!stage1
 linux-headers-7.2.6-76070206-generic-64k deb devel optional arch=arm64 profile=!stage1
 linux-headers-generic deb kernel optional arch=amd64,arm64,powerpc,ppc64el,s390x profile=!stage1
 linux-image-7.2.6-76070206-generic deb kernel optional arch=amd64,arm64,ppc64el profile=!stage1
 linux-image-7.2.6-76070206-generic-64k deb kernel optional arch=arm64 profile=!stage1
 linux-image-7.2.6-76070206-generic-64k-dbgsym deb devel optional arch=arm64 profile=!stage1
 linux-image-7.2.6-76070206-generic-dbgsym deb devel optional arch=amd64,arm64,ppc64el profile=!stage1
 linux-image-generic deb kernel optional arch=amd64,arm64,powerpc,ppc64el,s390x profile=!stage1
 linux-image-unsigned-7.2.6-76070206-generic deb kernel optional arch=s390x profile=!stage1
 linux-image-unsigned-7.2.6-76070206-generic-dbgsym deb devel optional arch=s390x profile=!stage1
 linux-lib-rust-7.2.6-76070206-generic deb devel optional arch=amd64 profile=!stage1
 linux-lib-rust-7.2.6-76070206-generic-64k deb devel optional arch=amd64 profile=!stage1
 linux-libc-dev deb devel optional arch=amd64,arm64,ppc64el,riscv64,s390x
 linux-modules-7.2.6-76070206-generic deb kernel optional arch=amd64,arm64,ppc64el,s390x profile=!stage1
 linux-modules-7.2.6-76070206-generic-64k deb kernel optional arch=arm64 profile=!stage1
 linux-perf deb devel optional arch=amd64,arm64,ppc64el,riscv64,s390x
 linux-source-7.2.6 deb devel optional arch=all profile=!stage1
 linux-system76 deb kernel optional arch=amd64,arm64 profile=!stage1
 linux-tools-7.2.6-76070206 deb devel optional arch=amd64,arm64,ppc64el,s390x profile=!stage1
 linux-tools-7.2.6-76070206-generic deb devel optional arch=amd64,arm64,ppc64el,s390x profile=!stage1
 linux-tools-7.2.6-76070206-generic-64k deb devel optional arch=arm64 profile=!stage1
 linux-tools-common deb kernel optional arch=all profile=!stage1
 linux-tools-generic deb kernel optional arch=amd64,arm64,powerpc,ppc64el,s390x profile=!stage1
 linux-tools-host deb kernel optional arch=all profile=!stage1
Checksums-Sha1:
 56395af639ff5f38a087c8f29ed4cbbeac1543a5 236700068 linux_7.2.6-76070206.202609141300~1792433904~24.04~141c6f5.tar.xz
Checksums-Sha256:
 fa92c4023427e44b0daf952612863f68418ae0445e37a6968ccc6f803214cb00 236700068 linux_7.2.6-76070206.202609141300~1792433904~24.04~141c6f5.tar.xz
Files:
 e0dded93e6d2655f8c9b4d38b83a8bd8 236700068 linux_7.2.6-76070206.202609141300~1792433904~24.04~141c6f5.tar.xz
Ubuntu-Compatible-Signing: ubuntu/4 pro/3 livepatch:2025 kmod:2025

-----BEGIN PGP SIGNATURE-----

iQJGBAEBCgAwFiEEY8Rt8BQNc4lhQp9OIE3YrsM6ev8FAmrWfm4SHGluZm9Ac3lz
dGVtNzYuY29tAAoJECBN2K7DOnr/6mEP/0V2WR8b12ZXaxl66NnsmvLxMGpAbNMP
r6mwg/sSoqfK2sMj+CUJyeQkyupTYFLmopkVU41SMOS/LEDNpFOXstiAYGyuXtp1
yDZSVg5eqJ4U1Srtvt8wONsGSuRKBDgPowwKBXXEzGqcvK6UkAPVVKNBJS9fDCHY
4CudGiOgNE/UEXmeye2g3svm+0w/cXkxetAfh7683n3DxJUhFCJAhOfHy01scZcI
/eK1zMYTRcpUIyLe6ygsa5aBn/iCHPEcawnC2PZun40x17qdHXL1x7m0G5/+oi4f
wo99/xzWZgJgCR8ZkyTpWfVws4n5r1T8WRrRD9VByIszy1Sqm/NMlLdnftq23Btf
TXYOY2lcbzjjZZ5VTzxbrrpjRrwjE75M+++p30+3DmfATEL1cpR6a4J58feY9JEm
snN9cD7Cu2t7WnBVIwu5iHJ/Ibv/1d0R9khWZMp0dJMcGXOYekzJ9o7dDwy08kfx
IO6oAGZM+lXVE0Z21kfmoyP4GHeXx+CYX8AjhGhltMDpqf6VXGvjd+dtt/3MJUYg
cinLcikGrUAkzSzsMq3Bm6RKcVmJYr4Gl7Gv43/sivfd/DpcrH/k2/Vwjrd/sgPV
mWfDWDQta0mhcnq45cCInPAr+i5S6X8NDcKTeiosQKK6oeyA0rMnGwHk5Zz/cL63
xuZTUOgXTwz+
=dTir
-----END PGP SIGNATURE-----
