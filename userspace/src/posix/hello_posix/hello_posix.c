#include <string.h>
#include <unistd.h>

const char *hello = "Hello World from C\n";

int main(void) {
  write(1, hello, strlen(hello));
  return 0;
}
