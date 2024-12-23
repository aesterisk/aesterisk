# Aesterisk

![Development Phase](https://img.shields.io/badge/Phase-Development-%23fde047?style=for-the-badge)
![GitHub last commit](https://img.shields.io/github/last-commit/aesterisk/aesterisk?style=for-the-badge&color=%23fde047)
![GitHub License](https://img.shields.io/github/license/aesterisk/aesterisk?style=for-the-badge&color=%23fde047)

`Aesterisk` is a server management platform
designed for hobbyists and startups to help
manage and deploy applications and services
on their own hosts, no matter if that's a VPS,
a server rack on-site, or an old desktop PC
you want to put to good use.

## What?

`Aesterisk` gives you a beautiful interface to manage and deploy your applications and services,
giving you the power to control your machines whenever and whereever you are.

With `Aesterisk`, you can deploy your project in a single click,
or automatically when you push to your production branch,
while keeping everything running on your machine.
You're in control over your machines when you're using `Aesterisk`.

## Why?

Well, there are a couple of different users that would benefit from `Aesterisk`.

### Hobbyists

You want a fun little website for yourself, but you have
no idea on how to set one up? With `Aesterisk`, spinning up
a server has never been easier. Install the `Aesterisk Daemon`
on an old machine that has been sitting in your garage, and
simply click on a server template to deploy, like Nginx for
web hosting, or a Minecraft server for you and your friends.

### Developers

`Aesterisk` has a first-class integration with GitHub to make
the developer experience of your project as smooth as possible.
Link your repository, and let the magic happen. Pushed a fix on the
master branch? `Aesterisk` is automatically deploying your application,
and you can see the process live, right on GitHub.

`Aesterisk` is built on top of Docker, which is loved by the
development community all around the world, so your existing
Dockerfiles and images will continue to work flawlessly.

### Startups

`Aesterisk` is a huge time saver when collaborating,
which is crucial for startups as they need to move fast.
`Aesterisk` automatically runs tests on each commit and pull request,
and makes a new deployment on each pull request to let others
immediately see how your changes affect the application,
skipping the proccess of manually checking out the branch,
compiling and running the server. (Which often happens on multiple
machines, which is a waste of processing power. Stay efficient!)

`Aesterisk` is a good way to keep multiple machines in one
management system, no matter where your VPS is located at or
what company is running them, or if it's a server rack on-sit
or an old PC, it works exactly the same.

What `Aesterisk` does *not* do and will not try to, is to
become a Kubernetes replacement. This means that while you
can manage multiple machines in `Aesterisk`, **you** choose
what container runs where. If you want an orchestrator, just
use Kubernetes. `Aesterisk` is designed for hobbyists with a
spare machine or maybe some dedicated servers, or a small
company/startup with low traffic and no load balancing needs.
(Now, you can implement a load balancer in `Aesterisk` of
course, but it won't be the load balancer you'd want if you're
looking into Kubernetes.)

## How?

Alright, enough talking. If you simply want to get started or
learn more about `Aesterisk`s features, head on over to our
website. (TODO: update when launching)

If you want to know more about how it all works,
here's the brief summary: (feel free to check out
the source code if you're interested!)

(TODO: explain hehe)

## Licensing

© 2024 Valter Sporrenstrand ([@yolocat-dev](https://github.com/yolocat-dev)).

This software is licensed under the GNU Affero General Public License (AGPL) version 3. See [LICENSE](https://github.com/aesterisk/aesterisk/tree/main/LICENSE) for the complete terms.
