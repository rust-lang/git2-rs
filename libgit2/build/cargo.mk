OPTS = -DTHREADSAFE=ON -DBUILD_SHARED_LIBS=OFF \
       -DBUILD_CLAR=OFF -DCMAKE_C_FLAGS=-fPIC \
       -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_EXAMPLES=OFF
all:
	cmake build/libgit2 -G "Unix Makefiles" -B"$(OUT_DIR)" $(OPTS)
	make -C "$(OUT_DIR)" -j10
