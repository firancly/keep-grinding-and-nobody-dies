/*
  KEEP GRINDING AND NOBODY DIES (I GUESS)
  I/O BRIDGE FIRMWARE - USB SERIAL VERSION

  All game logic (timer, mistakes, modules, sabotage events) lives in the
  Rust/Tauri backend, connected over the USB serial cable. This firmware
  only:
    - debounces the 4 buttons and reports their stable state, plus every
      press/release edge (with this device's own millis() timestamps, so
      hold-duration math stays exact)
    - reports the 4 wires' raw connected/cut state
    - drives the physical 7-segment display on command

  Wire protocol (115200 baud, newline-terminated lines):
    ESP32 -> Rust, every ~50ms, one line of JSON:
      {"espMillis":123456,"buttonsStable":[false,true,false,false],
       "wiresCut":[false,false,true,false],
       "edges":[{"button":0,"kind":"press","t":123001}]}
    Rust -> ESP32, whenever the countdown display should change:
      DISPLAY 173

  BUTTONS, each wired from GPIO through the button to GND:
    B1 -> GPIO14  (B1 = physical button index 0 = the game-start trigger)
    B2 -> GPIO27
    B3 -> GPIO26
    B4 -> GPIO33

  WIRES, each wired from GPIO through the removable wire to GND:
    W1 -> GPIO25
    W2 -> GPIO13
    W3 -> GPIO16
    W4 -> GPIO17

  COUNTER:
    SDI / DIN -> GPIO32
    SCLK      -> GPIO18
    LOAD / CS -> GPIO19
*/

#include <string.h>
#include <stdlib.h>

// ============================================================
// Hardware settings
// ============================================================

const int BUTTON_PINS[4] = {14, 27, 26, 33};
const int WIRE_PINS[4] = {25, 13, 16, 17};

const int DIN_PIN = 32;
const int CLK_PIN = 18;
const int LOAD_PIN = 19;

const uint32_t BUTTON_DEBOUNCE_MS = 35;
const uint32_t BROADCAST_INTERVAL_MS = 50;

const uint8_t DIGIT_SEGMENTS[10] = {
  0b11000000,
  0b11111001,
  0b10100100,
  0b10110000,
  0b10011001,
  0b10010010,
  0b10000011,
  0b11111000,
  0b10000000,
  0b10011000
};

const uint8_t SEGMENT_BLANK = 0b11111111;

bool buttonRawDown[4] = {false, false, false, false};
bool buttonStableDown[4] = {false, false, false, false};
bool buttonPressed[4] = {false, false, false, false};
bool buttonReleased[4] = {false, false, false, false};
uint32_t buttonRawChangedAt[4] = {0, 0, 0, 0};

// ============================================================
// Button press/release edges pending the next broadcast
// ============================================================

struct PendingEdge {
  uint8_t button;
  bool isPress;
  uint32_t t;
};

const int MAX_PENDING_EDGES = 16;
PendingEdge pendingEdges[MAX_PENDING_EDGES];
int pendingEdgeCount = 0;

void pushEdge(uint8_t button, bool isPress, uint32_t t) {
  if (pendingEdgeCount < MAX_PENDING_EDGES) {
    pendingEdges[pendingEdgeCount].button = button;
    pendingEdges[pendingEdgeCount].isPress = isPress;
    pendingEdges[pendingEdgeCount].t = t;
    pendingEdgeCount++;
  }
  // If the buffer is full, further edges within this broadcast window are
  // dropped - at a 50ms broadcast interval this would need 16+ button
  // transitions from one human in under 50ms, which doesn't happen.
}

// ============================================================
// Counter display
// ============================================================

void writeDisplayBytes(
  uint8_t leftDigit,
  uint8_t middleDigit,
  uint8_t rightDigit
) {
  digitalWrite(LOAD_PIN, LOW);
  shiftOut(DIN_PIN, CLK_PIN, MSBFIRST, rightDigit);
  shiftOut(DIN_PIN, CLK_PIN, MSBFIRST, middleDigit);
  shiftOut(DIN_PIN, CLK_PIN, MSBFIRST, leftDigit);
  digitalWrite(LOAD_PIN, HIGH);
  digitalWrite(LOAD_PIN, LOW);
}

void displayInit() {
  digitalWrite(DIN_PIN, LOW);
  digitalWrite(CLK_PIN, LOW);
  digitalWrite(LOAD_PIN, LOW);

  writeDisplayBytes(
    SEGMENT_BLANK,
    SEGMENT_BLANK,
    SEGMENT_BLANK
  );
}

void showNumber(int value) {
  value = constrain(value, 0, 999);

  int hundreds = value / 100;
  int tens = (value / 10) % 10;
  int ones = value % 10;

  writeDisplayBytes(
    DIGIT_SEGMENTS[hundreds],
    DIGIT_SEGMENTS[tens],
    DIGIT_SEGMENTS[ones]
  );
}

// ============================================================
// Button debounce
// ============================================================

void updateButtons(uint32_t now) {
  for (int i = 0; i < 4; i++) {
    buttonPressed[i] = false;
    buttonReleased[i] = false;

    bool rawDown = digitalRead(BUTTON_PINS[i]) == LOW;

    if (rawDown != buttonRawDown[i]) {
      buttonRawDown[i] = rawDown;
      buttonRawChangedAt[i] = now;
    }

    if (
      now - buttonRawChangedAt[i] >= BUTTON_DEBOUNCE_MS &&
      buttonStableDown[i] != buttonRawDown[i]
    ) {
      buttonStableDown[i] = buttonRawDown[i];

      if (buttonStableDown[i]) {
        buttonPressed[i] = true;
        pushEdge(i, true, now);
      } else {
        buttonReleased[i] = true;
        pushEdge(i, false, now);
      }
    }
  }
}

// ============================================================
// Serial command reading (non-blocking line accumulator)
// ============================================================

char commandLineBuffer[64];
size_t commandLineLength = 0;

void handleCommandLine(const char* line) {
  if (strncmp(line, "DISPLAY ", 8) == 0) {
    int value = atoi(line + 8);
    showNumber(value);
  }
}

void pollSerialCommands() {
  while (Serial.available() > 0) {
    char c = (char)Serial.read();

    if (c == '\n' || c == '\r') {
      if (commandLineLength > 0) {
        commandLineBuffer[commandLineLength] = '\0';
        handleCommandLine(commandLineBuffer);
        commandLineLength = 0;
      }
    } else if (commandLineLength < sizeof(commandLineBuffer) - 1) {
      commandLineBuffer[commandLineLength] = c;
      commandLineLength++;
    }
  }
}

// ============================================================
// State broadcast
// ============================================================

uint32_t lastBroadcastAt = 0;
// Sized generously for the base fields plus all MAX_PENDING_EDGES entries
// (~44 bytes each) with headroom to spare - avoids any risk of the offset
// arithmetic below going negative/wrapping if the buffer were ever tight.
char broadcastBuffer[1024];

void maybeBroadcastState(uint32_t now) {
  if (now - lastBroadcastAt < BROADCAST_INTERVAL_MS) {
    return;
  }
  lastBroadcastAt = now;

  int offset = snprintf(
    broadcastBuffer, sizeof(broadcastBuffer),
    "{\"espMillis\":%lu,\"buttonsStable\":[%s,%s,%s,%s],\"wiresCut\":[%s,%s,%s,%s],\"edges\":[",
    (unsigned long)now,
    buttonStableDown[0] ? "true" : "false",
    buttonStableDown[1] ? "true" : "false",
    buttonStableDown[2] ? "true" : "false",
    buttonStableDown[3] ? "true" : "false",
    digitalRead(WIRE_PINS[0]) == HIGH ? "true" : "false",
    digitalRead(WIRE_PINS[1]) == HIGH ? "true" : "false",
    digitalRead(WIRE_PINS[2]) == HIGH ? "true" : "false",
    digitalRead(WIRE_PINS[3]) == HIGH ? "true" : "false"
  );

  for (int i = 0; i < pendingEdgeCount && offset >= 0 && offset < (int)sizeof(broadcastBuffer) - 64; i++) {
    offset += snprintf(
      broadcastBuffer + offset, sizeof(broadcastBuffer) - offset,
      "%s{\"button\":%d,\"kind\":\"%s\",\"t\":%lu}",
      i == 0 ? "" : ",",
      pendingEdges[i].button,
      pendingEdges[i].isPress ? "press" : "release",
      (unsigned long)pendingEdges[i].t
    );
  }

  snprintf(broadcastBuffer + offset, sizeof(broadcastBuffer) - offset, "]}");

  Serial.println(broadcastBuffer);
  pendingEdgeCount = 0;
}

// ============================================================
// Setup and main loop
// ============================================================

void setup() {
  Serial.begin(115200);
  delay(500);

  for (int i = 0; i < 4; i++) {
    pinMode(BUTTON_PINS[i], INPUT_PULLUP);

    bool down = digitalRead(BUTTON_PINS[i]) == LOW;
    buttonRawDown[i] = down;
    buttonStableDown[i] = down;
    buttonRawChangedAt[i] = millis();

    pinMode(WIRE_PINS[i], INPUT_PULLUP);
  }

  pinMode(DIN_PIN, OUTPUT);
  pinMode(CLK_PIN, OUTPUT);
  pinMode(LOAD_PIN, OUTPUT);

  displayInit();
}

void loop() {
  uint32_t now = millis();

  updateButtons(now);
  pollSerialCommands();
  maybeBroadcastState(now);
}
