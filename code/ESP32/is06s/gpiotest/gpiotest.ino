// GPIO サイクルテスト
// 出力のたびに毎回設定

#include <driver/gpio.h>

int pins[] = {18, 5, 15, 27, 14, 12};
int count = 6;

void setup() {
  Serial.begin(115200);
  Serial.println("GPIO CYCLE TEST v3");
}

void loop() {
  for (int i = 0; i < count; i++) {
    gpio_num_t pin = (gpio_num_t)pins[i];

    // 毎回リセット＆設定
    gpio_reset_pin(pin);
    gpio_set_direction(pin, GPIO_MODE_OUTPUT);
    gpio_set_pull_mode(pin, GPIO_FLOATING);

    Serial.printf("GPIO %d HIGH\n", pins[i]);
    gpio_set_level(pin, 1);
    delay(3000);
    gpio_set_level(pin, 0);
  }
}
