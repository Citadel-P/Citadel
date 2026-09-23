window.onload = function () {
  window.ui = SwaggerUIBundle({
    .../* CITADEL_CONFIG */,
    dom_id: "#swagger-ui",
    presets: [SwaggerUIBundle.presets.apis, SwaggerUIStandalonePreset],
    plugins: [SwaggerUIBundle.plugins.DownloadUrl, function PreferMergePatch() {
      return {
        wrapComponents: {
          contentType: (Original, system) => function ContentType(props) {
            // Swagger selects the first media type when this control mounts.
            // Prefer merge-patch when advertised; keep JSON selectable.
            const contentTypes = props.contentTypes?.sortBy(
              type => type === "application/merge-patch+json" ? 0 : 1,
            );
            return system.React.createElement(Original, { ...props, contentTypes });
          },
        },
      };
    }],
    layout: "StandaloneLayout",
  });
};
