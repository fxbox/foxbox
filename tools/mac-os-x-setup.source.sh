# This script must be sourced. Please run `source tools/mac-os-x-setup.sh`

# openssl and sqlite libraries are already installed on OS X base system. That's
# why we need explicitely specify the directories that brew installed.
# TODO: Remove sqlite once pkg_config is able to not bubble up a system path
# For more information, see https://github.com/fxbox/foxbox/issues/414
export DEP_OPENSSL_INCLUDE="$(brew --prefix openssl)/include"
export OPENSSL_LIB_DIR="$(brew --prefix openssl)/lib"
export SQLITE3_LIB_DIR="$(brew --prefix sqlite)/lib"
