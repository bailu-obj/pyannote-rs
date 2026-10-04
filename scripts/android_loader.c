#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc != 2 && argc != 4) {
        fprintf(stderr, "usage: loader LIBPYANNOTE [LIBORT MODEL]\n");
        return 2;
    }
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
    int (*check)(const char *, const char *) = dlsym(library, "pyannote_android_load");
    if (!check) { fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }
    int status = check(argc == 4 ? argv[2] : "", argc == 4 ? argv[3] : "");
    printf("Android shared library load: %s\n", status == 0 ? "PASS" : "FAIL");
    fflush(NULL);
    if (argc == 2) {
        dlclose(library);
        return status;
    }
    // Retain the process-wide ORT environment, as an Android app does.
    // This checks loading and session disposal, not ORT's exit destructors.
    _Exit(status);
}
