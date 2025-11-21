/**
 * A collection of regular expression constants used for validation throughout the application.
 */
export class Constants {
  /**
   * Hostname + port
   * e.g., 'my-host:3000', 'example.com:8080'
   */
  static validHostnameWithPort =
    '([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)' + '(\\.([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?))*' + ':[0-9]+';

  /**
   * IPv4 + port
   * e.g., '192.168.1.1:8080', '127.0.0.1:3000'
   */
  static validIpWithPort =
    '((25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])\\.){3}' + '(25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])' + ':[0-9]+';

  /**
   * Hostname OR IPv4, both with port
   */
  static validHostOrIp = `^(${Constants.validHostnameWithPort}|${Constants.validIpWithPort})$`;

  /**
   * Simple identifier (letters, numbers, hyphens, underscores)
   * e.g., 'my-container', 'test1', 'my-app-2'
   */
  static validNameIdentifier = '^[a-zA-Z0-9-_]+$';
}
