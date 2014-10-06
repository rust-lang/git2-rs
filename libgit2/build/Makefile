ifneq ($(findstring i686,$(TARGET)),)
FLAGS=-m32
else ifneq ($(findstring x86_64,$(TARGET)),)
FLAGS=-m64
endif

ifeq ($(findstring mingw,$(TARGET)),)
SEP=:
else
SEP=;
endif

OPTS = -DTHREADSAFE=ON -DBUILD_SHARED_LIBS=OFF \
       -DBUILD_CLAR=OFF \
       -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_EXAMPLES=OFF \
       -DCMAKE_C_FLAGS="$(FLAGS) -fPIC"

export CMAKE_PREFIX_PATH := $(DEP_LIBSSH2_STATIC_SYS_OUT_DIR)$(SEP)$(CMAKE_PREFIX_PATH)


# If libgit2 exists locally, don't do anything. Otherwise go and build curl
all:
	pkg-config --atleast-version=0.21.0 libgit2 || $(MAKE) build

build:
	cmake libgit2 -G "Unix Makefiles" -B"$(OUT_DIR)" $(OPTS)
	make -C "$(OUT_DIR)" -j$(NUM_JOBS)
