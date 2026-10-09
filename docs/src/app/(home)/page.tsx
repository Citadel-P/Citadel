import type { Metadata } from "next";
import Image from "next/image";
import Link from "next/link";
import {
  ArrowRight,
  ArrowUpRight,
  Bell,
  Boxes,
  GitBranch,
  HardDrive,
  Network,
  Server,
  ShieldCheck,
  Terminal,
  Workflow,
} from "lucide-react";
import { ProductPreview } from "@/components/home/product-preview";
import { docsAssetUrl, gitConfig } from "@/lib/shared";
import styles from "./page.module.css";

export const metadata: Metadata = {
  title: "Citadel — Your Docker control plane",
  description:
    "Deploy, observe, and automate Docker containers and Swarm environments with Citadel. Explore the live demo and get started with the documentation.",
};

const resources = [
  {
    icon: Server,
    title: "Connect your environments",
    description:
      "Bring local Docker hosts, remote Agents, and Edge connections into one view.",
    href: "/docs/resources/platforms",
    link: "Platforms",
  },
  {
    icon: Boxes,
    title: "Deploy on your terms",
    description:
      "Run individual containers or Compose stacks. Use the web editor or keep configuration in Git.",
    href: "/docs/resources/stacks",
    link: "Deployments & stacks",
  },
  {
    icon: Network,
    title: "Make Swarm manageable",
    description:
      "Manage services, nodes, configs, and secrets, with node agents for worker-level visibility.",
    href: "/docs/resources/platforms/docker-swarm",
    link: "Docker Swarm",
  },
  {
    icon: GitBranch,
    title: "Build from your source",
    description:
      "Build container images from Git and publish to your registries. Team adds execution on external build pools.",
    href: "/docs/resources/builds",
    link: "Builds & build pools",
  },
  {
    icon: Workflow,
    title: "Put routine work on a schedule",
    description:
      "Connect actions into repeatable workflows. Run them manually, or use Team for schedules and webhook triggers.",
    href: "/docs/resources/automation-actions",
    link: "Automation",
  },
  {
    icon: HardDrive,
    title: "Plan for recovery",
    description:
      "Back up the Citadel control plane and Docker volumes, with policies for retention and restore.",
    href: "/docs/resources/backups",
    link: "Backups",
  },
];

export default function HomePage() {
  return (
    <main className={styles.home}>
      <section className={styles.hero} aria-labelledby="hero-title">
        <div className={styles.heroGrid} aria-hidden="true" />
        <div className={styles.heroContent}>
          <p className={styles.eyebrow}>
            <span /> Self-hosted. Built in Rust. Yours to operate.
          </p>
          <h1 id="hero-title">
            Your Docker estate.
            <br />
            <span>Under your control.</span>
          </h1>
          <p className={styles.intro}>
            Deploy, observe, and automate containers and Swarm environments.
            <br className={styles.desktopBreak} /> One control plane, from your
            first host to your next cluster.
          </p>
          <div className={styles.actions}>
            <Link
              href="/docs/getting-started/quick-start"
              className={styles.primary}
            >
              Get started <ArrowRight size={17} aria-hidden="true" />
            </Link>
            <a
              href="https://demo.citadelplane.com"
              className={styles.secondary}
            >
              Explore the live demo{" "}
              <ArrowUpRight size={17} aria-hidden="true" />
            </a>
          </div>
          <p className={styles.heroNote}>
            Docker Standalone <span aria-hidden="true">/</span> Docker Compose{" "}
            <span aria-hidden="true">/</span> Docker Swarm
          </p>
        </div>
        <div className={styles.previewWrap}>
          <ProductPreview />
        </div>
      </section>

      <section className={styles.section} aria-labelledby="resources-title">
        <div className={styles.sectionHeading}>
          <div>
            <p className={styles.eyebrow}>The tools behind your workloads</p>
            <h2 id="resources-title">From deployment to day two.</h2>
          </div>
          <Link className={styles.textLink} href="/docs/resources">
            Explore all resources <ArrowRight size={16} aria-hidden="true" />
          </Link>
        </div>
        <div className={styles.resourceGrid}>
          {resources.map(({ icon: Icon, title, description, href, link }) => (
            <Link href={href} key={title} className={styles.resource}>
              <Icon
                className={styles.resourceIcon}
                size={23}
                strokeWidth={1.6}
                aria-hidden="true"
              />
              <h3>{title}</h3>
              <p>{description}</p>
              <span className={styles.resourceLink}>
                {link}
                <ArrowUpRight size={16} aria-hidden="true" />
              </span>
            </Link>
          ))}
        </div>
      </section>

      <section
        className={`${styles.section} ${styles.gitSection}`}
        aria-labelledby="git-title"
      >
        <div className={styles.gitIntro}>
          <p className={styles.eyebrow}>Keep the source of truth in Git</p>
          <h2 id="git-title">
            One repository.
            <br />
            Multiple stacks.
          </h2>
          <p>
            Organize your applications in a monorepo. Give each stack its own
            Compose path and watched files, then use Team’s webhook-triggered
            deployments to apply relevant changes.
          </p>
          <Link className={styles.textLink} href="/docs/resources/stacks/git">
            Set up a Git stack <ArrowRight size={16} aria-hidden="true" />
          </Link>
        </div>
        <div
          className={styles.repoDiagram}
          aria-label="Example monorepo with two independently configured stacks"
        >
          <div className={styles.repoHeader}>
            <GitBranch size={18} aria-hidden="true" />
            <span>infrastructure</span>
            <code>main</code>
          </div>
          <div className={styles.repoRow}>
            <div>
              <span className={styles.treeLine} aria-hidden="true">
                ├─
              </span>
              <span>
                apps / website / <strong>compose.yaml</strong>
              </span>
            </div>
            <span className={styles.stackLabel}>
              <Boxes size={14} aria-hidden="true" /> Website stack
            </span>
          </div>
          <div className={styles.repoRow}>
            <div>
              <span className={styles.treeLine} aria-hidden="true">
                └─
              </span>
              <span>
                apps / counter / <strong>compose.yaml</strong>
              </span>
            </div>
            <span className={styles.stackLabel}>
              <Boxes size={14} aria-hidden="true" /> Counter stack
            </span>
          </div>
          <div className={styles.repoFooter}>
            <span>Commit</span>
            <ArrowRight size={14} aria-hidden="true" />
            <span>Webhook</span>
            <ArrowRight size={14} aria-hidden="true" />
            <span>Deploy changed stacks</span>
          </div>
        </div>
      </section>

      <section
        className={`${styles.section} ${styles.operations}`}
        aria-label="Everyday operations"
      >
        <Link href="/docs/operations/platform-monitoring">
          <Terminal size={21} aria-hidden="true" />
          <div>
            <h3>See what’s happening</h3>
            <p>Inspect resources, follow logs, and monitor usage.</p>
          </div>
          <ArrowUpRight size={16} aria-hidden="true" />
        </Link>
        <Link href="/docs/resources/alert-rules">
          <Bell size={21} aria-hidden="true" />
          <div>
            <h3>Know when it needs attention</h3>
            <p>Send alerts to the channels your team uses.</p>
          </div>
          <ArrowUpRight size={16} aria-hidden="true" />
        </Link>
        <Link href="/docs/guides/access-control">
          <ShieldCheck size={21} aria-hidden="true" />
          <div>
            <h3>Give access with intent</h3>
            <p>Use built-in roles, with custom access control available in Team.</p>
          </div>
          <ArrowUpRight size={16} aria-hidden="true" />
        </Link>
      </section>

      <section
        id="licensing"
        className={`${styles.section} ${styles.licensing}`}
        aria-labelledby="licensing-title"
      >
        <div className={styles.sectionHeading}>
          <div>
            <p className={styles.eyebrow}>Choose the capabilities you need</p>
            <h2 id="licensing-title">License and editions</h2>
          </div>
          <a
            className={styles.textLink}
            href={`https://github.com/${gitConfig.user}/${gitConfig.repo}/blob/${gitConfig.licenseRef}/LICENSE`}
          >
            Read license terms <ArrowUpRight size={16} aria-hidden="true" />
          </a>
        </div>
        <p>
          Citadel is source-available under Elastic License 2.0. Community is
          free for personal self-hosting and internal business production use.
          Team adds paid capabilities enabled by a signed product key.
        </p>
        <p>
          ELv2 is not an OSI-approved open-source license. Use, modification,
          and redistribution are subject to its terms. A Team key does not
          grant hosting or reseller permission.
        </p>
        <div className={styles.editionGrid}>
          <article className={styles.edition} aria-labelledby="community-title">
            <p className={styles.eyebrow}>Free</p>
            <h3 id="community-title">Community</h3>
            <p>
              Manage Docker and Swarm, deploy containers and Compose stacks,
              inspect logs, and run manual builds and recovery workflows.
              No product-license-enforced limits on hosts, containers, stacks,
              or users. Local authentication, MFA, and existing OIDC are included;
              no vendor account or paid activation is required.
            </p>
            <Link className={styles.textLink} href="/docs/overview/licensing#community">
              Explore Community <ArrowRight size={16} aria-hidden="true" />
            </Link>
          </article>
          <article className={styles.edition} aria-labelledby="team-title">
            <p className={styles.eyebrow}>Paid capabilities</p>
            <h3 id="team-title">Team</h3>
            <p>
              Add scheduled and webhook-triggered operations, custom access
              control, advanced alerts, continuous guardrails, and execution
              through external build pools. Scheduled backups and
              webhook-triggered deployments require Team. The capabilities in
              your product key determine which paid workflows are enabled.
            </p>
            <Link className={styles.textLink} href="/docs/overview/licensing#edition-comparison">
              Compare editions <ArrowRight size={16} aria-hidden="true" />
            </Link>
          </article>
        </div>
        <p className={styles.commitment}>
          Established Community capabilities stay free in later supported
          releases. Read the{" "}
          <Link className={styles.textLink} href="/docs/overview/community-commitment">
            Community commitment
          </Link>{" "}
          for its scope and change policy. This product commitment does not
          change the source license.
        </p>
      </section>

      <section className={styles.startSection} aria-labelledby="start-title">
        <Image
          src={docsAssetUrl("/logo.svg")}
          alt=""
          width={52}
          height={52}
          aria-hidden="true"
        />
        <p className={styles.eyebrow}>Start with one host. Grow from there.</p>
        <h2 id="start-title">Make yourself at home.</h2>
        <p>
          Install Citadel with Docker Compose, or take a look around the live
          demo first.
        </p>
        <div className={styles.actions}>
          <Link
            className={styles.primary}
            href="/docs/getting-started/quick-start"
          >
            Install Citadel <ArrowRight size={17} aria-hidden="true" />
          </Link>
          <a className={styles.secondary} href="https://demo.citadelplane.com">
            Open demo <ArrowUpRight size={17} aria-hidden="true" />
          </a>
        </div>
        <p className={styles.demoLogin}>
          Demo viewer: <code>demo</code>
          <span aria-hidden="true"> / </span>
          <code>citadel-demo-2026</code>
        </p>
      </section>

      <footer className={styles.footer}>
        <Link className={styles.footerBrand} href="/">
          <Image
            src={docsAssetUrl("/logo.svg")}
            alt=""
            width={22}
            height={22}
          />
          Citadel<span>Your Docker control plane.</span>
        </Link>
        <nav aria-label="Footer">
          <Link href="/docs">Documentation</Link>
          <Link href="/docs/reference/api">API</Link>
          <Link href="/docs/overview/licensing">Licensing</Link>
          <a href="https://preview.citadelplane.com">Dev preview</a>
          <a href={`https://github.com/${gitConfig.user}/${gitConfig.repo}`}>
            GitHub <ArrowUpRight size={13} aria-hidden="true" />
          </a>
        </nav>
      </footer>
    </main>
  );
}
