#include "Operator.h"

void Operator::setMode(OperatorMode mode) {
  mode_ = mode;
  Serial.printf("[OPERATOR] Mode changed to %s\n", modeName());
}

const char* Operator::modeName() const {
  switch (mode_) {
    case OperatorMode::PROVISION:   return "PROVISION";
    case OperatorMode::RUN:         return "RUN";
    case OperatorMode::MAINTENANCE: return "MAINTENANCE";
    case OperatorMode::FAILSAFE:    return "FAILSAFE";
    default:                        return "UNKNOWN";
  }
}

volatile bool* Operator::getLockPtr(ResourceLock resource) {
  switch (resource) {
    case ResourceLock::I2C:         return &lockI2C_;
    case ResourceLock::WIFI:        return &lockWiFi_;
    case ResourceLock::HTTP_CLIENT: return &lockHttpClient_;
    case ResourceLock::HTTP_SERVER: return &lockHttpServer_;
    case ResourceLock::OTA:         return &lockOta_;
    case ResourceLock::BLE:         return &lockBle_;
    default:                        return nullptr;
  }
}

bool Operator::lock(ResourceLock resource, unsigned long timeoutMs) {
  volatile bool* lockPtr = getLockPtr(resource);
  if (!lockPtr) return false;

  unsigned long start = millis();

  while (*lockPtr) {
    if (timeoutMs > 0 && (millis() - start) >= timeoutMs) {
      return false;  // タイムアウト
    }
    delay(1);  // 待機
  }

  *lockPtr = true;
  return true;
}

void Operator::unlock(ResourceLock resource) {
  volatile bool* lockPtr = getLockPtr(resource);
  if (lockPtr) {
    *lockPtr = false;
  }
}

bool Operator::isLocked(ResourceLock resource) const {
  // const版なのでキャストが必要
  Operator* self = const_cast<Operator*>(this);
  volatile bool* lockPtr = self->getLockPtr(resource);
  return lockPtr ? *lockPtr : false;
}

void Operator::unlockAll() {
  lockI2C_ = false;
  lockWiFi_ = false;
  lockHttpClient_ = false;
  lockHttpServer_ = false;
  lockOta_ = false;
  lockBle_ = false;
}
