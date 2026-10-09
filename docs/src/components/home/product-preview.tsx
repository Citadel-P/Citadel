"use client";

import { useState } from "react";
import Image from "next/image";
import { ArrowUpRight, Boxes, Network, Server } from "lucide-react";
import { docsAssetUrl } from "@/lib/shared";
import styles from "@/app/(home)/page.module.css";

const views = [
  {
    name: "Platforms",
    icon: Server,
    file: "platforms",
    alt: "Citadel Platforms showing Docker and Swarm hosts, connection status, and resource usage.",
    caption: "Your Docker environments, together in one view.",
  },
  {
    name: "Stacks",
    icon: Boxes,
    file: "stacks",
    alt: "Citadel Compose stack details with services, deployment controls, and logs.",
    caption: "Configuration, services, and deployment history for each stack.",
  },
  {
    name: "Swarm services",
    icon: Network,
    file: "swarm-services",
    alt: "Citadel Swarm services with replica counts and service status.",
    caption: "A closer look at the services running across your Swarm.",
  },
];

export function ProductPreview() {
  const [selected, setSelected] = useState(0);
  const view = views[selected];
  return (
    <figure className={styles.preview}>
      <div className={styles.previewToolbar}>
        <div
          className={styles.previewTabs}
          role="group"
          aria-label="Choose a product screenshot"
        >
          {views.map(({ name, icon: Icon }, index) => (
            <button
              key={name}
              type="button"
              aria-pressed={selected === index}
              aria-controls="product-screenshot"
              onClick={() => setSelected(index)}
            >
              <Icon size={15} aria-hidden="true" />
              {name}
            </button>
          ))}
        </div>
        <a href="https://demo.citadelplane.com" className={styles.previewDemo}>
          Try it live <ArrowUpRight size={14} aria-hidden="true" />
        </a>
      </div>
      <div id="product-screenshot" className={styles.screenshot}>
        <Image
          className={view.file === "platforms" ? "dark:hidden" : ""}
          src={docsAssetUrl(`/screenshots/${view.file}.png`)}
          alt={view.alt}
          width={1440}
          height={900}
          sizes="(max-width: 1200px) 94vw, 1120px"
          priority={selected === 0}
        />
        {view.file === "platforms" && (
          <Image
            className="hidden dark:block"
            src={docsAssetUrl("/screenshots/platforms-dark.png")}
            alt={view.alt}
            width={1440}
            height={900}
            sizes="(max-width: 1200px) 94vw, 1120px"
            priority
          />
        )}
      </div>
      <figcaption>
        <span aria-live="polite">{view.caption}</span>
        <span>Demonstration data</span>
      </figcaption>
    </figure>
  );
}
