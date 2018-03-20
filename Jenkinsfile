node {
    def bin
    def app

    stage('Clone repository') {
        checkout scm
        sh 'git submodule update --init --recursive'
    }

    stage('Build app') {
        sh 'cp -f docker/Dockerfile.build Dockerfile'
        bin = docker.build("storiqateam/stq-stores-interm:${env.BRANCH_NAME}")
        sh 'rm -f Dockerfile'
    }

    stage('Get binaries') {
        sh "docker run -i --rm --volume ${env.WORKSPACE}:/mnt/ storiqateam/stq-stores-interm:${env.BRANCH_NAME} cp -pf /app/target/release/stores /mnt/"
        sh "docker run -i --rm --volume ${env.WORKSPACE}:/mnt/ storiqateam/stq-stores-interm:${env.BRANCH_NAME} cp -pf /usr/local/cargo/bin/diesel /mnt/"
        sh "docker run -i --rm --volume ${env.WORKSPACE}:/mnt/ storiqateam/stq-stores-interm:${env.BRANCH_NAME} cp -rpf /app/migrations /mnt/ || mkdir migrations"
    }

    stage('Build app image') {
        sh 'cp -f docker/Dockerfile.run Dockerfile'
        app = docker.build("storiqateam/stq-stores:${env.BRANCH_NAME}")
        sh 'sudo /bin/rm -f Dockerfile stores diesel'
        sh 'sudo /bin/rm -rf migrations'
    }

    stage('Push image') {
        docker.withRegistry('https://registry.hub.docker.com', '4ca2ddae-a205-45f5-aaf7-333789c385cd') {
            app.push("${env.BRANCH_NAME}${env.BUILD_NUMBER}")
            app.push("${env.BRANCH_NAME}")
        }
    }
}
