// The result queue: one producer, one consumer, no lock on either side.

#include "slate_reaper.h"

#include <atomic>
#include <new>

namespace slate {

struct ResultQueue::Impl {
  explicit Impl(std::size_t capacity)
      : slots(new (std::nothrow) Result[capacity == 0 ? 1 : capacity]),
        capacity(capacity == 0 ? 1 : capacity) {}

  ~Impl() { delete[] slots; }

  Result* slots = nullptr;
  std::size_t capacity = 1;
  std::atomic<std::size_t> head{0};  // consumer index
  std::atomic<std::size_t> tail{0};  // producer index
};

ResultQueue::ResultQueue(std::size_t capacity) : impl_(new (std::nothrow) Impl(capacity)) {}

ResultQueue::~ResultQueue() { delete impl_; }

bool ResultQueue::push(const Result& result) {
  if (impl_ == nullptr || impl_->slots == nullptr) {
    return false;
  }
  const std::size_t tail = impl_->tail.load(std::memory_order_relaxed);
  const std::size_t head = impl_->head.load(std::memory_order_acquire);
  if (tail - head >= impl_->capacity) {
    return false;
  }
  impl_->slots[tail % impl_->capacity] = result;
  impl_->tail.store(tail + 1, std::memory_order_release);
  return true;
}

bool ResultQueue::pop(Result* out) {
  if (impl_ == nullptr || impl_->slots == nullptr) {
    return false;
  }
  const std::size_t head = impl_->head.load(std::memory_order_relaxed);
  const std::size_t tail = impl_->tail.load(std::memory_order_acquire);
  if (head == tail) {
    return false;
  }
  *out = impl_->slots[head % impl_->capacity];
  impl_->head.store(head + 1, std::memory_order_release);
  return true;
}

std::size_t ResultQueue::size() const {
  if (impl_ == nullptr) {
    return 0;
  }
  const std::size_t head = impl_->head.load(std::memory_order_acquire);
  const std::size_t tail = impl_->tail.load(std::memory_order_acquire);
  return tail - head;
}

}  // namespace slate
