PWD = $(CURDIR)
OPTS = -DTHREADSAFE=ON -DBUILD_SHARED_LIBS=OFF \
       -DCMAKE_BUILD_TYPE=Release -DBUILD_CLAR=OFF
all:
	cmake build/libgit2 -G "Unix Makefiles" -B"$(OUT_DIR)" $(OPTS)
	make -C "$(OUT_DIR)" -j10
