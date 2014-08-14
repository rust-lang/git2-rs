ifneq ($(findstring i686,$(TARGET)),)
FLAGS=-m32
else
FLAGS=-m64
endif
OPTS = -DTHREADSAFE=ON -DBUILD_SHARED_LIBS=OFF \
       -DBUILD_CLAR=OFF \
       -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_EXAMPLES=OFF \
       -DCMAKE_C_FLAGS="$(FLAGS) -fPIC"
all:
	cmake build/libgit2 -G "Unix Makefiles" -B"$(OUT_DIR)" $(OPTS)
	make -C "$(OUT_DIR)" -j10
