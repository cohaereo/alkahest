/*!
 * Minimal Umbra memory allocator stub
 * Provides just enough functionality for runtime queries
 */

#include "umbraMemory.hpp"
#include <cstdlib>

namespace Umbra {

// Default allocator implementation
class DefaultAllocator : public Allocator {
public:
  void *allocate(size_t size, const char * = NULL) override {
    return malloc(size);
  }

  void deallocate(void *ptr) override { free(ptr); }
};

// Static default allocator instance
static DefaultAllocator g_defaultAllocator;

// Global allocator pointer
Allocator *g_allocator = &g_defaultAllocator;

Allocator *getAllocator(void) { return g_allocator; }

void setAllocator(Allocator *allocator) {
  if (allocator) {
    g_allocator = allocator;
  } else {
    g_allocator = &g_defaultAllocator;
  }
}

// Thread-local storage stubs (not used in this implementation)
namespace Thread {
int allocTls(void) { return 0; }
void freeTls(int) {}
void setTlsValue(int, void *) {}
void *getTlsValue(int) { return NULL; }
} // namespace Thread

} // namespace Umbra
