#pragma once
#include <Arduino.h>

struct AraneaRegisterResult {
  bool ok = false;
  String cic_code;       // 6 digits (string)
  String stateEndpoint;  // URL
  String error;
};

class AraneaRegister {
public:
  AraneaRegisterResult registerDevice(); // scaffold only
};
