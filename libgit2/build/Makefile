ifneq ($(findstring i686,$(TARGET)),)
FLAGS=-m32
else
FLAGS=-m64
endif
OPTS = -DTHREADSAFE=ON -DBUILD_SHARED_LIBS=OFF \
       -DBUILD_CLAR=OFF \
       -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_EXAMPLES=OFF \
       -DCMAKE_C_FLAGS="$(FLAGS) -fPIC"

LIBSSH2_ROOT = $(wildcard $(OUT_DIR)/../libssh2-static-sys*)
export PKG_CONFIG_PATH := $(LIBSSH2_ROOT)/lib/pkgconfig:$(PKG_CONFIG_PATH)
export CMAKE_PREFIX_PATH := $(LIBSSH2_ROOT):$(CMAKE_PREFIX_PATH)

all:
	cmake libgit2 -G "Unix Makefiles" -B"$(OUT_DIR)" $(OPTS)
	make -C "$(OUT_DIR)" -j10
