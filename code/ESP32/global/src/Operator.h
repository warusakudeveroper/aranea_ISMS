#pragma once
#include <Arduino.h>

enum class OperatorMode { PROVISION, RUN, MAINTENANCE, FAILSAFE };

class Operator {
public:
  void setMode(OperatorMode mode);
  OperatorMode mode() const;

private:
  OperatorMode mode_ = OperatorMode::PROVISION;
};
