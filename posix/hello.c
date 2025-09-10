#include <string.h>
#include <unistd.h>

const char *text = "Hello World!\n";
const char *warning = "Foo! Bar!\n";

int main() {
  write(STDOUT_FILENO, text, strlen(text));
  write(STDERR_FILENO, warning, strlen(warning));
  return 0;
}
