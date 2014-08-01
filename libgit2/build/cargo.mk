PWD = $(CURDIR)
OPTS = -DTHREADSAFE=ON -DBUILD_SHARED_LIBS=OFF \
       -DCMAKE_BUILD_TYPE=Release -DBUILD_CLAR=OFF
all:
	cd $(DEPS_DIR)
	(cd $(DEPS_DIR) && cmake $(PWD)/build/libgit2 $(OPTS))
	(cd $(DEPS_DIR) && cmake --build . -- -j10)
