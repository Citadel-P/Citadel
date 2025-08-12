/**
 * A collection of regular expression constants used for validation throughout the application.
 */
export class Constants {
  /**
   * A regular expression for validating a hostname with a port number.
   * e.g., 'my-host:3000', 'example.com:8080'
   */
  static validHostnameWithPort =
    '^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9]).)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9-]*[A-Za-z0-9]):[0-9]+$';

  /**
   * A regular expression for validating an IP address with a port number.
   * e.g., '192.168.1.1:8080', '127.0.0.1:3000'
   */
  static validIpWithPort =
    '^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]).){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]):[0-9]+$';

  /**
   * A regular expression for validating either a hostname or an IP address, both with a port number.
   */
  static validHostOrIp = `${Constants.validHostnameWithPort}|${Constants.validIpWithPort}`;

  /**
   * A regular expression for validating a name identifier, which can contain letters, numbers, and hyphens.
   * e.g., 'my-container', 'test1', 'my-app-2'
   */
  static validNameIdentifier = '^[a-zA-Z0-9-]+$';
}
