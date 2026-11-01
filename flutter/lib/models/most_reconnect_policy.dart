/// Bounded recovery after an established session loses its transport.
class MostReconnectPolicy {
  int _attempts = 0;

  int? nextDelaySeconds() {
    if (_attempts >= 3) return null;
    _attempts++;
    return 5;
  }

  void reset() {
    _attempts = 0;
  }
}
