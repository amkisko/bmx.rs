# FreeBSD port for bmx
# Submit to the Ports tree (e.g. sysutils/bmx) or use as a local port.
# Before first build: run "make cargo-crates" in the port directory, then add
# the CARGO_CRATES output to this Makefile or to Makefile.crates (recommended).
# See: https://docs.freebsd.org/en/books/porters-handbook/special/#using-cargo

PORTNAME=	bmx
DISTVERSIONPREFIX=	v
DISTVERSION=	0.1.3
CATEGORIES=	sysutils
MAINTAINER=	contact@kiskolabs.com
COMMENT=	Command-line tool that installs, builds, and runs software from source repositories
WWW=		https://github.com/amkisko/bmx.rs
LICENSE=	MIT
LICENSE_FILE=	${WRKSRC}/LICENSE.md

USES=		cargo
USE_GITHUB=	yes
GH_ACCOUNT=	amkisko
GH_PROJECT=	bmx.rs

# Optional: place CARGO_CRATES in Makefile.crates if the list is long
# .include "${.CURDIR}/Makefile.crates"
.include <bsd.port.mk>
