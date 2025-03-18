# To Run
Clone this repository with any of the listed methods, or in whichever alternative way you wish 
- via [Github Desktop](https://desktop.github.com/download/), with the URL 'https://github.com/Jarten0/short_flight' (See [here](https://docs.github.com/en/desktop/adding-and-cloning-repositories/cloning-and-forking-repositories-from-github-desktop#cloning-a-repository))
- via the [gh CLI tool](https://github.com/cli/cli?tab=readme-ov-file#installation), with the command ```gh repo clone Jarten0/short_flight```
- via cloning with [git](https://git-scm.com/downloads), with the command ```git clone https://github.com/Jarten0/short_flight```

Then download a build of the game. As of writing it is distributed via [Github Releases](https://github.com/Jarten0/short_flight/releases), but in the future it might be by some other way.
Insert it into the top level of the repository, alongside the `assets` folder.

The build will likely be outdated by some amount, so follow the steps below in [To Build](https://github.com/Jarten0/short_flight?tab=readme-ov-file#to-build) to build the game yourself and try out the most recent additions. 

# To Build
First clone the repository, as according to the first step of [To Run](https://github.com/Jarten0/short_flight?tab=readme-ov-file#to-run).

Then follow this excerpt from [Bevy Quick Start - Setup](https://bevyengine.org/learn/quick-start/getting-started/setup/#rust-setup) in order to install everything needed for building a bevy game.
(Up to date as of 2025-03-18)

>    ### Installing Rust
>    Bevy relies heavily on improvements in the Rust language and compiler.
>    As a result, the Minimum Supported Rust Version (MSRV) is "the latest stable release" of Rust.
>
>    Install Rust by following the [Rust Getting Started Guide](https://www.rust-lang.org/learn/get-started).
>
>    Once this is done, you should have the ```rustc``` compiler and the ```cargo``` build system installed in your path.
>
>    ### Installing OS Dependencies
>
>    <details>
>    <summary>
>    Linux
>    </summary>
>
>    Follow the instructions at [Linux Dependencies](https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md)
>    </details>
>
>    <details>
>    <summary>
>    Windows
>    </summary>
>
>    * Run the [Visual Studio 2019 build tools installer](https://visualstudio.microsoft.com/thank-you-downloading-visual-studio/?sku=BuildTools&rel=16)
>    * For easy setup, select the ```Desktop development with C++``` workload in the installer.
>    * For a minimal setup, follow these steps:
>        1. In the installer, navigate to `Individual components`
>        2. Select the latest `MSVC` for your architecture and version of Windows
>        3. Select the latest `Windows SDK` for your version of Windows
>        4. Select the `C++ CMake tools` for Windows component
>        5. Install the components
>    </details>
>
>    <details>
>    <summary> MacOS </summary>
>
>    Install the Xcode command line tools with `xcode-select --install` or the [Xcode app](https://apps.apple.com/en/app/xcode/id497799835)
>    </details>

Once you've finished with that, you can simply type the command `cargo run` in the directory of the repository, which will run the game. 
