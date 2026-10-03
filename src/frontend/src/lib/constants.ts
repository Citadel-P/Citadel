/**
 * A collection of regular expression constants used for validation throughout the application.
 */
export class Constants {
  // --- Core Components (Extracted for Clarity) ---

  /**
   * Hostname only (e.g., 'example.com', 'my-registry.net')
   */
  static validHostname = '([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)' + '(\\.([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?))*';

  /**
   * IPv4 only (e.g., '192.168.1.1', '127.0.0.1')
   */
  static validIp = '((25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])\\.){3}' + '(25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])';

  /**
   * Port only (e.g., ':443', ':5000')
   */
  static validPortSuffix = ':[0-9]+';

  /**
   * Hostname OR IPv4, with an OPTIONAL port suffix.
   * e.g., 'docker.io', 'my-registry.com:5000', '192.168.1.1'
   */
  static validHostnameWithPort = `^(${Constants.validHostname}|${Constants.validIp})(${Constants.validPortSuffix})?$`;

  /**
   * IPv4 + port (Redundant, but matches original structure)
   */
  static validIpWithPort = Constants.validIp + Constants.validPortSuffix;

  /**
   * Hostname OR IPv4, both with port (Original, enforces port presence)
   */
  static validHostOrIp = `^(${Constants.validHostnameWithPort}|${Constants.validIpWithPort})$`;

  /**
   * Simple identifier (letters, numbers, hyphens, underscores)
   */
  static validNameIdentifier = '^[a-zA-Z0-9-_]+$';

  /**
   * Valid email or username
   */
  static validEmailOrName = '^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$|^[a-zA-Z0-9_-]+$';
}
