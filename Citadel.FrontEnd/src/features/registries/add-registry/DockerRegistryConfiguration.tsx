import { AlertMessage } from "@/components/ui/alert-message";

const DockerRegistry = () => {

    return (
        <div>
            <AlertMessage type="info">For information on how to generate a DockerHub Access Token, follow the <a className="hover:underline" href="https://docs.docker.com/security/for-developers/access-tokens/">dockerhub guide</a>.</AlertMessage>

        </div>
    )
}

export default DockerRegistry;