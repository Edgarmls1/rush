# Maintainer: edgar1macedosalazar@gmail.com
pkgname=rush
pkgver=1.0
pkgrel=1
pkgdesc="a rust based terminal"
arch=('x86_64')
url="https://github.com/Edgarmls1/rush"
license=('MIT')
depends=('gcc' 'rust')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/heads/main.tar.gz")
sha256sums=('SKIP')

build() {
    cd "rush-main"
    cargo build --release
}

package() {
    cd "rush-main"
    sudo install -Dm755 "target/release/reedit" "/usr/bin/"
}
