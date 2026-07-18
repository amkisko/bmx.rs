# Gentoo ebuild for bmx
# Copy to a local overlay under app-misc/bmx/
# For a full offline build, generate an ebuild with: cargo install cargo-ebuild && cargo ebuild
# (then use the generated ebuild which includes CARGO_CRATE_URIS)

EAPI=8

inherit cargo

DESCRIPTION="Command-line tool that installs, builds, and runs software from source repositories"
HOMEPAGE="https://github.com/amkisko/bmx.rs"
SRC_URI="https://github.com/amkisko/bmx.rs/archive/refs/tags/v${PV}.tar.gz -> ${P}.tar.gz"
S="${WORKDIR}/bmx.rs-${PV}"

LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64 ~arm64"

RDEPEND="dev-libs/openssl:="
DEPEND="${RDEPEND}"

src_install() {
	cargo_src_install
	einstalldocs
	dodoc LICENSE.md
}
