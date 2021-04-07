import chalk from "chalk";

export const Info = chalk.blueBright;
export const Success = chalk.greenBright;
export const Err = chalk.redBright;

export var verbose = false;

export const setVerbose = (v) => { verbose = v || false};

export const logMsg = (msg, depth) => verbose ? console.dir(msg, { depth: depth || 10}) : {};
