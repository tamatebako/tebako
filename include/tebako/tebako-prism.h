#include <fcntl.h>

static pm_string_init_result_t
tebako_string_file_init(pm_string_t *string, const char *filepath) {

    // Open the file for reading
    int fd = open(filepath, O_RDONLY);
    if (fd == -1) {
        return PM_STRING_INIT_ERROR_GENERIC;
    }

    // Stat the file to get the file size
    struct stat sb;
    if (fstat(fd, &sb) == -1) {
        close(fd);
        return PM_STRING_INIT_ERROR_GENERIC;
    }

    // Ensure it is a file and not a directory
    if (S_ISDIR(sb.st_mode)) {
        close(fd);
        return PM_STRING_INIT_ERROR_DIRECTORY;
    }

    // Check the size to see if it's empty
    size_t size = (size_t) sb.st_size;
    if (size == 0) {
        close(fd);
        const uint8_t source[] = "";
        *string = (pm_string_t) { .type = PM_STRING_CONSTANT, .source = source, .length = 0 };
        return PM_STRING_INIT_SUCCESS;
    }

    size_t length = (size_t) size;
    uint8_t *source = xmalloc(length);
    if (source == NULL) {
        close(fd);
        return PM_STRING_INIT_ERROR_GENERIC;
    }

    long bytes_read = (long) read(fd, source, length);
    close(fd);

    if (bytes_read == -1) {
        xfree(source);
        return PM_STRING_INIT_ERROR_GENERIC;
    }

    *string = (pm_string_t) { .type = PM_STRING_OWNED, .source = source, .length = length };
    return PM_STRING_INIT_SUCCESS;
}
