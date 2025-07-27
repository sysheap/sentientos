#include <string.h>
#include <unistd.h>

const char *text = "Hello World!\n";

int main() {
  write(STDOUT_FILENO, text, strlen(text));
  return 0;
}
