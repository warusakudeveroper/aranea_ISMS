#pragma once
#include <Arduino.h>

class LacisIDGenerator {
public:
  // productType: "001".."007" (3 chars)
  // productCode: "0096" (4 chars)
  // returns lacisId: "3" + productType + MAC(12HEX) + productCode (20 chars)
  String generate(const String& productType, const String& productCode);
};
